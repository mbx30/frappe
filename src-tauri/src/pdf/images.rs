use lopdf::{Document, Object};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct ImageResolutionFinding {
    pub page: usize,
    pub image_name: String,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub rendered_width_pts: f64,
    pub rendered_height_pts: f64,
    pub effective_dpi: f64,
    pub color_space: String,
    pub severity: String,
    pub message: String,
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

fn is_operator_char(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'*'
}

fn parse_content_operations(content: &[u8]) -> Vec<(String, Vec<f64>, Vec<Vec<u8>>)> {
    let len = content.len();
    let mut ops = Vec::new();
    let mut operands_num: Vec<f64> = Vec::new();
    let mut operands_name: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;

    while i < len {
        if is_whitespace(content[i]) {
            i += 1;
            continue;
        }
        if content[i] == b'%' {
            while i < len && content[i] != b'\n' && content[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        if content[i] == b'/' {
            let start = i;
            i += 1;
            while i < len && !is_whitespace(content[i]) && content[i] != b'%' {
                i += 1;
            }
            operands_name.push(content[start..i].to_vec());
            continue;
        }

        if content[i] == b'-' || content[i] == b'+' || is_digit(content[i]) || content[i] == b'.' {
            let start = i;
            if content[i] == b'-' || content[i] == b'+' {
                i += 1;
            }
            while i < len && is_digit(content[i]) {
                i += 1;
            }
            if i < len && content[i] == b'.' {
                i += 1;
                while i < len && is_digit(content[i]) {
                    i += 1;
                }
            }
            let s = std::str::from_utf8(&content[start..i]).unwrap_or("0");
            if let Ok(n) = s.parse::<f64>() {
                operands_num.push(n);
            }
            continue;
        }

        if is_operator_char(content[i]) {
            let start = i;
            while i < len && is_operator_char(content[i]) {
                i += 1;
            }
            let op = String::from_utf8_lossy(&content[start..i]).to_string();
            ops.push((
                op,
                std::mem::take(&mut operands_num),
                std::mem::take(&mut operands_name),
            ));
            continue;
        }

        if content[i] == b'(' {
            let mut depth = 0;
            while i < len {
                if content[i] == b'(' {
                    depth += 1;
                }
                if content[i] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                if content[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            continue;
        }

        if content[i] == b'<' && i + 1 < len && content[i + 1] != b'<' {
            while i < len && content[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    ops
}

fn get_resources<'a>(doc: &'a Document, page_id: (u32, u16)) -> Option<lopdf::Dictionary> {
    let page_dict = doc.get_dictionary(page_id).ok()?;
    page_dict.get(b"Resources").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })
}

fn find_xobject_dict(doc: &Document, resources: &lopdf::Dictionary) -> Option<lopdf::Dictionary> {
    resources.get(b"XObject").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })
}

fn collect_xobject_subtype(
    doc: &Document,
    xobject_dict: &lopdf::Dictionary,
    name: &[u8],
) -> Option<Vec<u8>> {
    xobject_dict.get(name).ok().and_then(|value| {
        let stream = match value {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
            _ => return None,
        };
        stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| n.to_vec())
    })
}

fn collect_form_xobject_stream(
    doc: &Document,
    xobject_dict: &lopdf::Dictionary,
    name: &[u8],
) -> Option<Vec<u8>> {
    xobject_dict.get(name).ok().and_then(|value| {
        let stream = match value {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
            _ => return None,
        };
        Some(stream.content.clone())
    })
}

fn collect_form_xobject_resources(
    doc: &Document,
    xobject_dict: &lopdf::Dictionary,
    name: &[u8],
) -> Option<lopdf::Dictionary> {
    xobject_dict.get(name).ok().and_then(|value| {
        let stream = match value {
            Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| o.as_stream().ok())?,
            _ => return None,
        };
        stream.dict.get(b"Resources").ok().and_then(|o| match o {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
            _ => None,
        })
    })
}

fn collect_image_xobjects(doc: &Document, page_id: (u32, u16)) -> Vec<(Vec<u8>, u32, u32, String)> {
    let mut images = Vec::new();
    let resources = match get_resources(doc, page_id) {
        Some(r) => r,
        None => return images,
    };
    let xobject_dict = match find_xobject_dict(doc, &resources) {
        Some(x) => x,
        None => return images,
    };
    for (name, value) in xobject_dict.iter() {
        let stream = match value {
            Object::Reference(id) => {
                match doc.get_object(*id).ok().and_then(|o| o.as_stream().ok()) {
                    Some(s) => s,
                    None => continue,
                }
            }
            _ => continue,
        };
        let subtype = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok());
        if subtype != Some(b"Image") {
            continue;
        }
        let width = stream
            .dict
            .get(b"Width")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0) as u32;
        let height = stream
            .dict
            .get(b"Height")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0) as u32;
        let color_space = stream
            .dict
            .get(b"ColorSpace")
            .ok()
            .and_then(|o| {
                o.as_name()
                    .map(|n| String::from_utf8_lossy(n).to_string())
                    .ok()
            })
            .unwrap_or_else(|| "Unknown".into());
        images.push((name.clone(), width, height, color_space));
    }
    images
}

fn process_content_stream(
    doc: &Document,
    content_bytes: &[u8],
    image_defs: &[(Vec<u8>, u32, u32, String)],
    page: usize,
    findings: &mut Vec<ImageResolutionFinding>,
    initial_ctm: [f64; 6],
    xobject_dict: &lopdf::Dictionary,
    depth: usize,
) {
    if depth > 10 {
        return;
    }

    let ops = parse_content_operations(content_bytes);
    let mut ctm = initial_ctm;
    let mut ctm_stack: Vec<[f64; 6]> = Vec::new();
    let mut seen_usages: std::collections::HashSet<(String, i64, i64)> =
        std::collections::HashSet::new();

    for (op, nums, names) in &ops {
        match op.as_str() {
            "q" => ctm_stack.push(ctm),
            "Q" => {
                if let Some(saved) = ctm_stack.pop() {
                    ctm = saved;
                }
            }
            "cm" => {
                if nums.len() >= 6 {
                    let [a, b, c, d, e, f] = [nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]];
                    let [a0, b0, c0, d0, e0, f0] = ctm;
                    ctm = [
                        a * a0 + b * c0,
                        a * b0 + b * d0,
                        c * a0 + d * c0,
                        c * b0 + d * d0,
                        e * a0 + f * c0 + e0,
                        e * b0 + f * d0 + f0,
                    ];
                }
            }
            "Do" => {
                if let Some(name_bytes) = names.first() {
                    let name = String::from_utf8_lossy(name_bytes);

                    // Check if this is a Form XObject (need to recurse)
                    let subtype = collect_xobject_subtype(doc, xobject_dict, name_bytes);
                    if subtype.as_deref() == Some(b"Form") {
                        if let Some(form_content) =
                            collect_form_xobject_stream(doc, xobject_dict, name_bytes)
                        {
                            let form_resources =
                                collect_form_xobject_resources(doc, xobject_dict, name_bytes)
                                    .unwrap_or_else(|| xobject_dict.clone());

                            // Create a synthetic xobject dict for recursion
                            let form_xobject_dict = find_xobject_dict(doc, &form_resources)
                                .unwrap_or_else(|| lopdf::Dictionary::new());

                            let form_image_defs = {
                                let mut imgs = Vec::new();
                                for (xn, xv) in form_xobject_dict.iter() {
                                    let obj = match xv {
                                        Object::Reference(id) => doc.get_object(*id).ok(),
                                        _ => continue,
                                    };
                                    let stream = match obj.and_then(|o| o.as_stream().ok()) {
                                        Some(s) => s,
                                        None => continue,
                                    };
                                    let subtype = stream
                                        .dict
                                        .get(b"Subtype")
                                        .ok()
                                        .and_then(|o| o.as_name().ok());
                                    if subtype == Some(b"Image") {
                                        let w = stream
                                            .dict
                                            .get(b"Width")
                                            .ok()
                                            .and_then(|o| o.as_i64().ok())
                                            .unwrap_or(0)
                                            as u32;
                                        let h = stream
                                            .dict
                                            .get(b"Height")
                                            .ok()
                                            .and_then(|o| o.as_i64().ok())
                                            .unwrap_or(0)
                                            as u32;
                                        let cs = stream
                                            .dict
                                            .get(b"ColorSpace")
                                            .ok()
                                            .and_then(|o| {
                                                o.as_name()
                                                    .map(|n| String::from_utf8_lossy(n).to_string())
                                                    .ok()
                                            })
                                            .unwrap_or_else(|| "Unknown".into());
                                        imgs.push((xn.clone(), w, h, cs));
                                    }
                                }
                                imgs
                            };

                            process_content_stream(
                                doc,
                                &form_content,
                                &form_image_defs,
                                page,
                                findings,
                                ctm,
                                &form_xobject_dict,
                                depth + 1,
                            );
                        }
                        continue;
                    }

                    // Compute the rendered width/height in points using the
                    // full CTM scale factors rather than just ctm[0]/ctm[3].
                    // For a rotated image, the CTM has non-zero off-diagonal
                    // entries (b, c); the true scale is the matrix's column
                    // magnitude: sqrt(a^2 + b^2) for x, sqrt(c^2 + d^2) for y.
                    // Using only ctm[0]/ctm[3] would under-report the size of
                    // rotated images and over-report their DPI. (#156)
                    let display_w_pts = (ctm[0] * ctm[0] + ctm[1] * ctm[1]).sqrt();
                    let display_h_pts = (ctm[2] * ctm[2] + ctm[3] * ctm[3]).sqrt();

                    for (img_name, pw, ph, cs) in image_defs {
                        let img_name_utf8 = String::from_utf8_lossy(img_name);
                        if img_name_utf8 == name || format!("/{}", img_name_utf8) == name {
                            let usage_key = (
                                img_name_utf8.to_string(),
                                (display_w_pts * 100.0) as i64,
                                (display_h_pts * 100.0) as i64,
                            );
                            if seen_usages.contains(&usage_key) {
                                break;
                            }
                            seen_usages.insert(usage_key);

                            if display_w_pts > 0.0 && display_h_pts > 0.0 {
                                let dpi_x = *pw as f64 / (display_w_pts / 72.0);
                                let dpi_y = *ph as f64 / (display_h_pts / 72.0);
                                let dpi = dpi_x.min(dpi_y);
                                let (severity, message) = if dpi < 150.0 {
                                    ("error".into(), format!("{:.0} DPI — below minimum", dpi))
                                } else if dpi < 300.0 {
                                    ("warning".into(), format!("{:.0} DPI — marginal", dpi))
                                } else if dpi > 1200.0 {
                                    (
                                        "info".into(),
                                        format!("{:.0} DPI — excessive (will slow RIP)", dpi),
                                    )
                                } else {
                                    ("ok".into(), format!("{:.0} DPI", dpi))
                                };
                                findings.push(ImageResolutionFinding {
                                    page,
                                    image_name: img_name_utf8.to_string(),
                                    pixel_width: *pw,
                                    pixel_height: *ph,
                                    rendered_width_pts: display_w_pts,
                                    rendered_height_pts: display_h_pts,
                                    effective_dpi: dpi,
                                    color_space: cs.clone(),
                                    severity,
                                    message,
                                });
                            } else {
                                let estimated_dpi = (*pw as f64).min(*ph as f64) / 2.0;
                                findings.push(ImageResolutionFinding {
                                    page,
                                    image_name: img_name_utf8.to_string(),
                                    pixel_width: *pw,
                                    pixel_height: *ph,
                                    rendered_width_pts: 0.0,
                                    rendered_height_pts: 0.0,
                                    effective_dpi: estimated_dpi,
                                    color_space: cs.clone(),
                                    severity: "info".into(),
                                    message: format!(
                                        "Est. {:.0} DPI (no transform found)",
                                        estimated_dpi
                                    ),
                                });
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn check_image_resolution(doc: &Document) -> Vec<ImageResolutionFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut findings = Vec::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let image_defs = collect_image_xobjects(doc, obj_id);

        let content_bytes = match doc.get_page_content(obj_id) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let resources = match get_resources(doc, obj_id) {
            Some(r) => r,
            None => continue,
        };
        let xobject_dict =
            find_xobject_dict(doc, &resources).unwrap_or_else(|| lopdf::Dictionary::new());

        process_content_stream(
            doc,
            &content_bytes,
            &image_defs,
            page,
            &mut findings,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            &xobject_dict,
            0,
        );
    }

    findings
}
