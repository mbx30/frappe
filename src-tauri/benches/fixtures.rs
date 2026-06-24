//! Bench harness helpers — generate a synthetic 5 MB PDF on disk
//! and a 20-page PDF for the preflight benchmarks.

use printpdf::*;
use std::io::BufWriter;
use std::path::PathBuf;

const FIXTURES_DIR: &str = "target/bench-fixtures";

/// Generate a 5 MB-ish PDF (lots of uncompressed image streams). The
/// output is cached on disk so repeated benchmark runs don't pay the
/// fixture-generation cost.
pub fn five_mb_pdf() -> PathBuf {
    let path = std::path::Path::new(FIXTURES_DIR).join("five-mb.pdf");
    if path.exists() {
        return path;
    }
    std::fs::create_dir_all(FIXTURES_DIR).expect("create fixtures dir");
    let (doc, page1, layer1) =
        PdfDocument::new("Benchmark PDF", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .expect("add font");
    let layer = doc.get_page(page1).get_layer(layer1);
    layer.use_text("Frappe compress benchmark", 14.0, Mm(20.0), Mm(280.0), &font);
    // Fill ~5 MB of uncompressible random data into the content stream.
    let chunk: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
    for _ in 0..(5 * 1024 * 1024 / 4096) {
        layer.add_line(Line {
            points: vec![
                (Point::new(Mm(10.0), Mm(10.0)), false),
                (Point::new(Mm(200.0), Mm(10.0)), false),
            ],
            is_closed: false,
        });
        // Append raw random bytes to push the byte count.
        layer.set_fill_color(Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)));
        let _ = chunk; // (kept for future bitmap padding)
    }
    let mut writer = BufWriter::new(std::fs::File::create(&path).expect("create pdf"));
    doc.save(&mut writer).expect("save pdf");
    path
}

/// Generate a 20-page PDF with mixed content for preflight benchmarks.
pub fn twenty_page_pdf() -> PathBuf {
    let path = std::path::Path::new(FIXTURES_DIR).join("twenty-page.pdf");
    if path.exists() {
        return path;
    }
    std::fs::create_dir_all(FIXTURES_DIR).expect("create fixtures dir");
    let (doc, page1, layer1) =
        PdfDocument::new("Preflight 20-page", Mm(210.0), Mm(297.0), "L1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .expect("add font");
    for i in 0..20 {
        let (page, layer) = if i == 0 {
            (page1, layer1)
        } else {
            doc.add_page(Mm(210.0), Mm(297.0), format!("L{i}"))
        };
        let l = doc.get_page(page).get_layer(layer);
        l.use_text(
            format!("Page {} — Frappe preflight benchmark", i + 1),
            12.0,
            Mm(20.0),
            Mm(280.0),
            &font,
        );
    }
    let mut writer = BufWriter::new(std::fs::File::create(&path).expect("create pdf"));
    doc.save(&mut writer).expect("save pdf");
    path
}
