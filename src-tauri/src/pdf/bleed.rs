use lopdf::Document;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct BleedFinding {
    pub page: usize,
    pub has_bleed_box: bool,
    pub bleed_top_mm: f64,
    pub bleed_right_mm: f64,
    pub bleed_bottom_mm: f64,
    pub bleed_left_mm: f64,
    pub min_required_mm: f64,
    pub severity: String,
    pub message: String,
}

const POINTS_TO_MM: f64 = 0.3528;

fn parse_rect(arr: &[lopdf::Object]) -> Option<(f64, f64, f64, f64)> {
    if arr.len() != 4 {
        return None;
    }
    let to_f64 = |o: &lopdf::Object| -> Option<f64> {
        match o {
            lopdf::Object::Integer(i) => Some(*i as f64),
            lopdf::Object::Real(r) => Some(*r as f64),
            _ => None,
        }
    };
    let x1 = to_f64(&arr[0])?;
    let y1 = to_f64(&arr[1])?;
    let x2 = to_f64(&arr[2])?;
    let y2 = to_f64(&arr[3])?;
    // Keep raw (x1, y1, x2, y2) so callers can inspect direction. Width/height
    // are still `abs()`-ed because a PDF rect is allowed to have x1 > x2 (the
    // spec defines it as two opposite corners, not strictly bottom-left then
    // top-right) — but for bleed *direction* comparisons callers need the
    // corner values themselves. See `check_bleed` for the directional logic.
    Some((x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs()))
}

fn get_box(
    page_dict: &lopdf::Dictionary,
    doc: &Document,
    key: &[u8],
) -> Option<(f64, f64, f64, f64)> {
    page_dict.get(key).ok().and_then(|o| match o {
        lopdf::Object::Array(a) => parse_rect(a),
        lopdf::Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_array().ok())
            .and_then(|a| parse_rect(a)),
        _ => None,
    })
}

pub fn check_bleed(doc: &Document, min_bleed_mm: f64) -> Vec<BleedFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut findings = Vec::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;
        let page_dict = match doc.get_dictionary(obj_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let trim_box = get_box(&page_dict, doc, b"TrimBox");
        let bleed_box = get_box(&page_dict, doc, b"BleedBox");

        let (has_bleed, top, right, bottom, left, severity, message) = match (bleed_box, trim_box) {
            (Some((bx, by, bw, bh)), Some((tx, ty, tw, th))) => {
                // Bleed is the margin *outside* the TrimBox. The previously
                // used `(by + bh) - (ty + th)).abs()` masked the case where
                // the TrimBox is *larger* than the BleedBox (a layout
                // violation: bleed is supposed to extend past trim, not the
                // other way around). Now we compute the signed difference so
                // a negative bleed on any side is reported as a violation.
                // (#173)
                //
                // We use the (min-x, min-y, w, h) returned by `parse_rect`,
                // so:
                //   bleed_top    = (by + bh) - (ty + th)   must be >= 0
                //   bleed_right  = (bx + bw) - (tx + tw)   must be >= 0
                //   bleed_bottom = ty - by                  must be >= 0
                //   bleed_left   = tx - bx                  must be >= 0
                let top_pts = (by + bh) - (ty + th);
                let right_pts = (bx + bw) - (tx + tw);
                let bottom_pts = ty - by;
                let left_pts = tx - bx;

                let top_mm = top_pts * POINTS_TO_MM;
                let right_mm = right_pts * POINTS_TO_MM;
                let bottom_mm = bottom_pts * POINTS_TO_MM;
                let left_mm = left_pts * POINTS_TO_MM;

                // Negative bleed on any side is a structural violation (TrimBox
                // extends past BleedBox), reported separately from "insufficient
                // bleed" so the operator can tell the difference.
                let any_negative =
                    top_pts < 0.0 || right_pts < 0.0 || bottom_pts < 0.0 || left_pts < 0.0;

                if any_negative {
                    let offending = [
                        ("top", top_mm),
                        ("right", right_mm),
                        ("bottom", bottom_mm),
                        ("left", left_mm),
                    ]
                    .iter()
                    .filter(|(_, v)| *v < 0.0)
                    .map(|(s, v)| format!("{s}={v:.1}mm"))
                    .collect::<Vec<_>>()
                    .join(", ");
                    (true, top_mm, right_mm, bottom_mm, left_mm,
                     "error".into(),
                     format!("TrimBox extends past BleedBox on: {offending} — bleed must be outside the trim area"))
                } else {
                    let min_side = top_mm.min(right_mm).min(bottom_mm).min(left_mm);
                    if min_side >= min_bleed_mm {
                        (true, top_mm, right_mm, bottom_mm, left_mm,
                         "ok".into(),
                         format!("Bleed OK — minimum {:.1}mm on all sides (top: {:.1}, right: {:.1}, bottom: {:.1}, left: {:.1})",
                                 min_bleed_mm, top_mm, right_mm, bottom_mm, left_mm))
                    } else {
                        (true, top_mm, right_mm, bottom_mm, left_mm,
                         "error".into(),
                         format!("Insufficient bleed: top={:.1}mm, right={:.1}mm, bottom={:.1}mm, left={:.1}mm (minimum {:.1}mm required)",
                                 top_mm, right_mm, bottom_mm, left_mm, min_bleed_mm))
                    }
                }
            }
            (Some((_bx, _by, bw, bh)), None) => (
                true,
                0.0,
                0.0,
                0.0,
                0.0,
                "warning".into(),
                format!(
                    "BleedBox present ({:.0}×{:.0} pts) but no TrimBox to validate against",
                    bw, bh
                ),
            ),
            (None, Some((_tx, _ty, tw, th))) => {
                let w_mm = tw * POINTS_TO_MM;
                let h_mm = th * POINTS_TO_MM;
                (false, 0.0, 0.0, 0.0, 0.0,
                 "error".into(),
                 format!("No BleedBox found. TrimBox is {:.0}×{:.0}mm — content may extend to edge without bleed margin",
                         w_mm, h_mm))
            }
            (None, None) => (
                false,
                0.0,
                0.0,
                0.0,
                0.0,
                "error".into(),
                "No BleedBox or TrimBox found on this page".into(),
            ),
        };

        findings.push(BleedFinding {
            page,
            has_bleed_box: has_bleed,
            bleed_top_mm: top,
            bleed_right_mm: right,
            bleed_bottom_mm: bottom,
            bleed_left_mm: left,
            min_required_mm: min_bleed_mm,
            severity,
            message,
        });
    }

    findings
}
