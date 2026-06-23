use lopdf::{Document, Object};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct PageBoxFinding {
    pub page: usize,
    pub box_type: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub severity: String,
    pub message: String,
}

fn obj_to_f64(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}

fn parse_rect(arr: &[Object]) -> Option<(f64, f64, f64, f64)> {
    if arr.len() != 4 {
        return None;
    }
    let x1 = obj_to_f64(&arr[0])?;
    let y1 = obj_to_f64(&arr[1])?;
    let x2 = obj_to_f64(&arr[2])?;
    let y2 = obj_to_f64(&arr[3])?;
    Some((x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs()))
}

fn get_box(
    doc: &Document,
    page_dict: &lopdf::Dictionary,
    key: &[u8],
) -> Option<(f64, f64, f64, f64)> {
    let obj = page_dict.get(key).ok()?;
    match obj {
        Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_array().ok())
            .and_then(|a| parse_rect(a)),
        other => other.as_array().ok().and_then(|a| parse_rect(a)),
    }
}

pub fn check_page_boxes(doc: &Document) -> Vec<PageBoxFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut findings = Vec::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let page_dict = match doc.get_dictionary(obj_id) {
            Ok(d) => d,
            Err(_) => {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "Page".into(),
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    severity: "error".into(),
                    message: "Could not read page dictionary".into(),
                });
                continue;
            }
        };

        let media_box = get_box(doc, &page_dict, b"MediaBox");
        let crop_box = get_box(doc, &page_dict, b"CropBox");
        let bleed_box = get_box(doc, &page_dict, b"BleedBox");
        let trim_box = get_box(doc, &page_dict, b"TrimBox");
        let art_box = get_box(doc, &page_dict, b"ArtBox");

        match media_box {
            Some((x, y, w, h)) => {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "MediaBox".into(),
                    x,
                    y,
                    w,
                    h,
                    severity: "ok".into(),
                    message: format!("MediaBox: {:.0} x {:.0} pts at ({:.0}, {:.0})", w, h, x, y),
                });
            }
            None => {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "MediaBox".into(),
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    severity: "error".into(),
                    message: "MediaBox missing — page may not render correctly".into(),
                });
                continue;
            }
        }

        let (mx, my, mw, mh) = media_box.unwrap();

        match &bleed_box {
            Some((x, y, w, h)) => {
                // BleedBox validation: it must be *contained within* the
                // MediaBox AND *contain* the TrimBox (per PDF spec §14.11.2
                // — BleedBox "shall be a box that includes the TrimBox plus
                // extra area for bleed"). The previous check only compared
                // sizes (`w <= tw`), which misses position violations where
                // the boxes are the right size but offset from each other.
                // (#174)
                let mut issues = Vec::new();

                // Containment within MediaBox: every corner of BleedBox must
                // lie inside MediaBox. We compare against the (min-x, min-y,
                // w, h) representation returned by parse_rect.
                if *x < mx || *y < my || x + w > mx + mw || y + h > my + mh {
                    issues.push("extends beyond MediaBox".to_string());
                }

                // Containment of TrimBox: TrimBox must lie entirely inside
                // BleedBox. Compare positions (not just sizes).
                if let Some((tx, ty, tw, th)) = &trim_box {
                    if *tx < *x || *ty < *y || tx + tw > x + w || ty + th > y + h {
                        issues.push("does not contain TrimBox".to_string());
                    }
                    // Keep the size check too — useful for a clearer message.
                    if *w < *tw || *h < *th {
                        issues.push("smaller than TrimBox".to_string());
                    }
                }
                let sev = if issues.is_empty() { "ok" } else { "warning" };
                let msg = if issues.is_empty() {
                    format!("BleedBox: {:.0} x {:.0} pts at ({:.0}, {:.0})", w, h, x, y)
                } else {
                    format!("BleedBox: {:.0} x {:.0} pts — {}", w, h, issues.join("; "))
                };
                findings.push(PageBoxFinding {
                    page,
                    box_type: "BleedBox".into(),
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                    severity: sev.into(),
                    message: msg,
                });
            }
            None => {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "BleedBox".into(),
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    severity: "info".into(),
                    message: "BleedBox absent — content may extend to edge without bleed".into(),
                });
            }
        }

        match &trim_box {
            Some((x, y, w, h)) => {
                if *x > mx + mw || *y > my + mh || x + w > mx + mw || y + h > my + mh {
                    findings.push(PageBoxFinding {
                        page,
                        box_type: "TrimBox".into(),
                        x: *x,
                        y: *y,
                        w: *w,
                        h: *h,
                        severity: "error".into(),
                        message: "TrimBox extends beyond MediaBox".into(),
                    });
                } else {
                    findings.push(PageBoxFinding {
                        page,
                        box_type: "TrimBox".into(),
                        x: *x,
                        y: *y,
                        w: *w,
                        h: *h,
                        severity: "ok".into(),
                        message: format!(
                            "TrimBox: {:.0} x {:.0} pts at ({:.0}, {:.0})",
                            w, h, x, y
                        ),
                    });
                }
            }
            None => {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "TrimBox".into(),
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    severity: "warning".into(),
                    message: "TrimBox missing — required for proper print registration".into(),
                });
            }
        }

        if let Some((x, y, w, h)) = &crop_box {
            if (w - mw).abs() > 0.5 || (h - mh).abs() > 0.5 {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "CropBox".into(),
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                    severity: "info".into(),
                    message: format!("CropBox differs from MediaBox: {:.0} x {:.0} pts", w, h),
                });
            }
        }

        if let Some((x, y, w, h)) = &art_box {
            let matches_trim = trim_box
                .as_ref()
                .map(|(_, _, tw, th)| (tw - *w).abs() < 0.5 && (th - *h).abs() < 0.5)
                .unwrap_or(false);
            if !matches_trim {
                findings.push(PageBoxFinding {
                    page,
                    box_type: "ArtBox".into(),
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                    severity: "info".into(),
                    message: format!("ArtBox: {:.0} x {:.0} pts at ({:.0}, {:.0})", w, h, x, y),
                });
            }
        }
    }

    findings
}
