use lopdf::Object;
use std::path::Path;

const MIN_BLEED_MM: f64 = 3.0;
const AMOUNT_MM: f64 = 5.0;
const POINTS_TO_MM: f64 = 0.3528;

fn make_pdf_with_trim_box(trim: [f64; 4], out_path: &Path) {
    let mut doc = lopdf::Document::new();
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(
        vec![
            (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
            (b"Parent".to_vec(), Object::Reference(pages_id)),
            (
                b"MediaBox".to_vec(),
                Object::Array(vec![
                    Object::Real(0.0 as f32),
                    Object::Real(0.0 as f32),
                    Object::Real(trim[2] as f32),
                    Object::Real(trim[3] as f32),
                ]),
            ),
            (
                b"TrimBox".to_vec(),
                Object::Array(vec![
                    Object::Real(trim[0] as f32),
                    Object::Real(trim[1] as f32),
                    Object::Real(trim[2] as f32),
                    Object::Real(trim[3] as f32),
                ]),
            ),
            (
                b"Resources".to_vec(),
                Object::Dictionary(lopdf::Dictionary::new()),
            ),
        ],
    )));
    let pages_dict = lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Object::Array(vec![Object::Reference(page_id)])),
        (b"Count".to_vec(), Object::Integer(1)),
    ]);
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    let catalog_id = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(
        vec![
            (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
            (b"Pages".to_vec(), Object::Reference(pages_id)),
        ],
    )));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(out_path).expect("save fixture");
}

#[test]
#[ignore = "requires PDF corpus directory"]
fn bleed_round_trip_adds_bleed_and_passes_recheck() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("pdf_corpus");
    if !corpus.exists() {
        eprintln!("PDF corpus directory not found at {:?}; skipping.", corpus);
        return;
    }
    // Create a 1-page PDF with a small TrimBox inside a larger MediaBox.
    let dir = std::env::temp_dir().join(format!("frappe_bleed_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.pdf");
    let output = dir.join("source_bleed.pdf");
    let trim = [50.0, 50.0, 250.0, 350.0];
    make_pdf_with_trim_box(trim, &source);

    // Run add_bleed via the public command surface.
    app_lib::commands::add_bleed(
        source.to_string_lossy().to_string(),
        AMOUNT_MM,
        output.to_string_lossy().to_string(),
    )
    .expect("add_bleed should succeed");

    // Source must not have been overwritten.
    assert!(source.exists(), "source should still exist after add_bleed");
    assert!(output.exists(), "output should exist after add_bleed");
    let output_bytes = std::fs::metadata(&output).unwrap().len();
    assert!(output_bytes > 0, "output should be non-empty");

    // Re-open the output and run check_bleed; expect the minimum to
    // meet MIN_BLEED_MM.
    let doc = lopdf::Document::load(&output).expect("re-open output");
    let findings = app_lib::pdf::bleed::check_bleed(&doc, MIN_BLEED_MM);
    assert_eq!(findings.len(), 1, "should have one page finding");
    let f = &findings[0];
    assert!(f.has_bleed_box, "BleedBox must be present after add_bleed");
    assert!(
        f.bleed_top_mm >= MIN_BLEED_MM,
        "top bleed should meet minimum, got {}",
        f.bleed_top_mm
    );
    assert!(
        f.bleed_right_mm >= MIN_BLEED_MM,
        "right bleed should meet minimum, got {}",
        f.bleed_right_mm
    );
    assert!(
        f.bleed_bottom_mm >= MIN_BLEED_MM,
        "bottom bleed should meet minimum, got {}",
        f.bleed_bottom_mm
    );
    assert!(
        f.bleed_left_mm >= MIN_BLEED_MM,
        "left bleed should meet minimum, got {}",
        f.bleed_left_mm
    );

    // Expected: original trim extended by AMOUNT_MM in points on
    // each side. 50 - 5/0.3528 ≈ 35.82; 250 + 5/0.3528 ≈ 264.18.
    let expected_expand_pts = AMOUNT_MM / POINTS_TO_MM;
    let new_bleed_box: Vec<f64> = doc
        .get_dictionary(doc.get_pages()[&1])
        .unwrap()
        .get(b"BleedBox")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|o| match o {
            Object::Integer(i) => *i as f64,
            Object::Real(r) => *r as f64,
            _ => panic!("unexpected type"),
        })
        .collect();
    assert_eq!(new_bleed_box.len(), 4);
    let tolerance = 0.5;
    assert!(
        (new_bleed_box[0] - (trim[0] - expected_expand_pts)).abs() < tolerance,
        "left should be {} got {}",
        trim[0] - expected_expand_pts,
        new_bleed_box[0]
    );
    assert!(
        (new_bleed_box[2] - (trim[2] + expected_expand_pts)).abs() < tolerance,
        "right should be {} got {}",
        trim[2] + expected_expand_pts,
        new_bleed_box[2]
    );

    let _ = std::fs::remove_dir_all(&dir);
}
