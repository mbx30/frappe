use lopdf::{Dictionary, Document, Object};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OverprintFinding {
    pub page: usize,
    pub object_context: String,
    pub overprint_stroke: bool,
    pub overprint_fill: bool,
    pub mode: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransparencyFinding {
    pub page: usize,
    pub ty: String,
    pub value: String,
    pub is_pdfx1a_violation: bool,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HiddenContentFinding {
    pub page: usize,
    pub ty: String,
    pub description: String,
    pub severity: String,
}

fn get_resources(doc: &Document, page_id: (u32, u16)) -> Option<Dictionary> {
    let page_dict = doc.get_dictionary(page_id).ok()?;
    page_dict.get(b"Resources").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })
}

fn find_extgstate(doc: &Document, resources: &Dictionary, name: &[u8]) -> Option<Dictionary> {
    let gs_dict = resources.get(b"ExtGState").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })?;
    match gs_dict.get(name) {
        Ok(Object::Dictionary(d)) => Some(d.clone()),
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

fn find_xobject_dict(doc: &Document, resources: &Dictionary) -> Option<Dictionary> {
    resources.get(b"XObject").ok().and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    })
}

fn collect_xobject_subtype(
    doc: &Document,
    xobject_dict: &Dictionary,
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
    xobject_dict: &Dictionary,
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
    xobject_dict: &Dictionary,
    name: &[u8],
) -> Option<Dictionary> {
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

pub fn check_overprint(doc: &Document) -> Vec<OverprintFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut findings = Vec::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let resources = match get_resources(doc, obj_id) {
            Some(r) => r,
            None => continue,
        };

        walk_overprint_stream(doc, obj_id, &resources, page, &mut findings, 0);
    }

    findings
}

fn walk_overprint_stream(
    doc: &Document,
    page_id: (u32, u16),
    resources: &Dictionary,
    page: usize,
    findings: &mut Vec<OverprintFinding>,
    depth: usize,
) {
    if depth > 10 {
        return;
    }

    let content_bytes = match doc.get_page_content(page_id) {
        Ok(c) => c,
        Err(_) => return,
    };

    let xobject_dict = find_xobject_dict(doc, resources).unwrap_or_else(|| Dictionary::new());
    let ops = parse_content_operations(&content_bytes);

    for (op, _nums, names) in &ops {
        if op == "gs" {
            if let Some(name_bytes) = names.first() {
                if let Some(gs_dict) = find_extgstate(doc, resources, name_bytes) {
                    let op_stroke = gs_dict
                        .get(b"OP")
                        .ok()
                        .and_then(|o| o.as_bool().ok())
                        .unwrap_or(false);
                    let op_fill = gs_dict
                        .get(b"op")
                        .ok()
                        .and_then(|o| o.as_bool().ok())
                        .unwrap_or(false);
                    let opm = gs_dict
                        .get(b"OPM")
                        .ok()
                        .and_then(|o| o.as_i64().ok())
                        .unwrap_or(0);

                    if op_stroke || op_fill {
                        // Check if the color just before was 0% ink
                        // We can't track the exact color here (needs full graphics state),
                        // so we flag all overprint usage and note the context
                        let sev = if op_stroke && op_fill {
                            "warning"
                        } else {
                            "info"
                        };
                        findings.push(OverprintFinding {
                            page,
                            object_context: format!("gs /{}", String::from_utf8_lossy(name_bytes)),
                            overprint_stroke: op_stroke,
                            overprint_fill: op_fill,
                            mode: if opm == 1 { "non-zero".into() } else { "knockout".into() },
                            severity: sev.into(),
                            message: format!(
                                "Overprint: stroke={}, fill={}, OPM={}. If overprinting on 0% ink, the underlying content will show through — verify white knockout.",
                                op_stroke, op_fill, opm
                            ),
                        });
                    }
                }
            }
        }

        if op == "Do" {
            if let Some(name_bytes) = names.first() {
                let subtype = collect_xobject_subtype(doc, &xobject_dict, name_bytes);
                if subtype.as_deref() == Some(b"Form") {
                    if let Some(form_content) =
                        collect_form_xobject_stream(doc, &xobject_dict, name_bytes)
                    {
                        let form_resources =
                            collect_form_xobject_resources(doc, &xobject_dict, name_bytes)
                                .unwrap_or_else(|| resources.clone());
                        // Walk form content inline
                        let form_ops = parse_content_operations(&form_content);
                        for (op2, _n2, names2) in &form_ops {
                            if op2 == "gs" {
                                if let Some(name2) = names2.first() {
                                    if let Some(gs2) = find_extgstate(doc, &form_resources, name2) {
                                        let os = gs2
                                            .get(b"OP")
                                            .ok()
                                            .and_then(|o| o.as_bool().ok())
                                            .unwrap_or(false);
                                        let of = gs2
                                            .get(b"op")
                                            .ok()
                                            .and_then(|o| o.as_bool().ok())
                                            .unwrap_or(false);
                                        if os || of {
                                            findings.push(OverprintFinding {
                                                page,
                                                object_context: format!("Form XObject /{} → gs /{}", String::from_utf8_lossy(name_bytes), String::from_utf8_lossy(name2)),
                                                overprint_stroke: os,
                                                overprint_fill: of,
                                                mode: "unknown".into(),
                                                severity: "warning".into(),
                                                message: "Overprint found inside Form XObject. Verify white knockout.".into(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn check_transparency(doc: &Document) -> Vec<TransparencyFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut findings = Vec::new();

    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let page_dict = match doc.get_dictionary(obj_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Check page-level Group dict
        if let Ok(group) = page_dict.get(b"Group") {
            if let Ok(group_dict) = group.as_dict() {
                if let Ok(s) = group_dict.get(b"S").and_then(|o| o.as_name()) {
                    if s == b"Transparency" {
                        let cs = group_dict
                            .get(b"CS")
                            .ok()
                            .and_then(|o| o.as_name().ok())
                            .map(|n| String::from_utf8_lossy(n).to_string())
                            .unwrap_or_else(|| "unknown".into());
                        findings.push(TransparencyFinding {
                            page,
                            ty: "page_group".into(),
                            value: format!("/Transparency /CS /{}", cs),
                            is_pdfx1a_violation: true,
                            severity: "info".into(),
                            message: format!("Page {} has a Transparency Group (CS={}). Required for live transparency.", page, cs),
                        });
                    }
                }
                // Check for /I key (isolated) and /K (knockout)
                if let Ok(true) = group_dict.get(b"I").and_then(|o| o.as_bool()) {
                    findings.push(TransparencyFinding {
                        page,
                        ty: "isolated_group".into(),
                        value: "/I true".into(),
                        is_pdfx1a_violation: true,
                        severity: "info".into(),
                        message: "Isolated transparency group (/I true). Used for compositing with transparent backdrop.".into(),
                    });
                }
            }
        }

        // Check ExtGState for ca/CA and BM
        let resources = match get_resources(doc, obj_id) {
            Some(r) => r,
            None => continue,
        };

        let gs_dict = match resources.get(b"ExtGState") {
            Ok(Object::Dictionary(d)) => d.clone(),
            Ok(Object::Reference(id)) => match doc.get_dictionary(*id) {
                Ok(d) => d.clone(),
                _ => continue,
            },
            _ => continue,
        };

        for (name, value) in gs_dict.iter() {
            let gs = match value {
                Object::Dictionary(d) => d.clone(),
                Object::Reference(id) => match doc.get_dictionary(*id) {
                    Ok(d) => d.clone(),
                    _ => continue,
                },
                _ => continue,
            };

            let gs_name = String::from_utf8_lossy(name).to_string();

            // ca — fill opacity
            if let Ok(ca) = gs.get(b"ca").and_then(|o| o.as_float()) {
                if ca < 1.0 {
                    findings.push(TransparencyFinding {
                        page,
                        ty: "fill_opacity".into(),
                        value: format!("ca={}", ca),
                        is_pdfx1a_violation: true,
                        severity: if ca < 0.5 { "warning".into() } else { "info".into() },
                        message: format!("ExtGState {}: fill opacity ca={} (<1.0). Live transparency detected — will need flattening for PDF/X-1a.", gs_name, ca),
                    });
                }
            }

            // CA — stroke opacity
            if let Ok(ca_val) = gs.get(b"CA").and_then(|o| o.as_float()) {
                if ca_val < 1.0 {
                    findings.push(TransparencyFinding {
                        page,
                        ty: "stroke_opacity".into(),
                        value: format!("CA={}", ca_val),
                        is_pdfx1a_violation: true,
                        severity: if ca_val < 0.5 { "warning".into() } else { "info".into() },
                        message: format!("ExtGState {}: stroke opacity CA={} (<1.0). Live transparency detected.", gs_name, ca_val),
                    });
                }
            }

            // BM — blend mode
            if let Ok(bm) = gs.get(b"BM").and_then(|o| o.as_name()) {
                let bm_str = String::from_utf8_lossy(bm).to_string();
                if bm_str != "Normal" && bm_str != "Compatible" {
                    findings.push(TransparencyFinding {
                        page,
                        ty: "blend_mode".into(),
                        value: format!("/{}", bm_str),
                        is_pdfx1a_violation: true,
                        severity: "warning".into(),
                        message: format!("ExtGState {}: blend mode /{} (non-default). Will not render correctly in PDF/X-1a.", gs_name, bm_str),
                    });
                }
            }
        }
    }

    findings
}

pub fn check_hidden_content(doc: &Document) -> Vec<HiddenContentFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let mut findings = Vec::new();

    // Check for Optional Content Groups (OCGs) in the catalog
    let catalog_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|r| r.as_reference().ok())
        .unwrap_or((0, 0));
    if let Ok(catalog) = doc.get_dictionary(catalog_ref) {
        if let Ok(ocprops) = catalog.get(b"OCProperties") {
            if let Ok(ocp_dict) = ocprops.as_dict() {
                // Check default state
                if let Ok(d) = ocp_dict.get(b"D").and_then(|o| o.as_dict()) {
                    // ON and OFF arrays
                    if let Ok(on) = d.get(b"ON").and_then(|o| o.as_array()) {
                        if on.is_empty() {
                            findings.push(HiddenContentFinding {
                                page: 0,
                                ty: "default_off_layers".into(),
                                description: "All Optional Content Groups (layers) are OFF by default. Content in these layers is hidden.".into(),
                                severity: "warning".into(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check each page for annotations that may be hidden
    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        let page = page_num + 1;

        let page_dict = match doc.get_dictionary(obj_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Check for off-page objects by examining MediaBox
        let media_box = page_dict
            .get(b"MediaBox")
            .ok()
            .and_then(|o| o.as_array().ok());
        if let Some(mb) = media_box {
            if mb.len() >= 4 {
                let w = mb[2].as_float().unwrap_or(0.0) - mb[0].as_float().unwrap_or(0.0);
                let h = mb[3].as_float().unwrap_or(0.0) - mb[1].as_float().unwrap_or(0.0);
                if w > 0.0 && h > 0.0 {
                    // Found a valid MediaBox — content stream off-page objects
                    // would require full path tracking. We note the potential.
                    // Check if BleedBox or ArtBox extends beyond MediaBox
                    let check_boxes: &[&[u8]] = &[b"BleedBox", b"ArtBox", b"TrimBox"];
                    for box_name in check_boxes {
                        if let Ok(bbox) = page_dict.get(box_name).and_then(|o| o.as_array()) {
                            if bbox.len() >= 4 {
                                let bx = bbox[0].as_float().unwrap_or(0.0);
                                let by = bbox[1].as_float().unwrap_or(0.0);
                                let bw = bbox[2].as_float().unwrap_or(0.0);
                                let bh = bbox[3].as_float().unwrap_or(0.0);
                                let bname = String::from_utf8_lossy(box_name);
                                if bx < mb[0].as_float().unwrap_or(0.0)
                                    || by < mb[1].as_float().unwrap_or(0.0)
                                    || bw > mb[2].as_float().unwrap_or(0.0)
                                    || bh > mb[3].as_float().unwrap_or(0.0)
                                {
                                    findings.push(HiddenContentFinding {
                                        page,
                                        ty: "off_page_box".into(),
                                        description: format!("{} extends beyond MediaBox on page {}. Content outside MediaBox is hidden or clipped.", bname, page),
                                        severity: "info".into(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for non-printing annotations
        if let Ok(annots) = page_dict.get(b"Annots").and_then(|o| o.as_array()) {
            let mut non_printing_count = 0;
            for annot_ref in annots {
                if let Object::Reference(id) = annot_ref {
                    if let Ok(annot) = doc.get_dictionary(*id) {
                        // Skip annotations that would cause a visible output problem
                        if let Ok(subtype) = annot.get(b"Subtype").and_then(|o| o.as_name()) {
                            if subtype == b"Link" || subtype == b"Widget" {
                                non_printing_count += 1;
                            }
                        }
                    }
                }
            }
            if non_printing_count > 0 {
                findings.push(HiddenContentFinding {
                    page,
                    ty: "non_printing_annotation".into(),
                    description: format!("Page {} has {} interactive annotation(s) (Link/Widget). These won't appear in printed output but modify visual appearance in the viewer.", page, non_printing_count),
                    severity: "info".into(),
                });
            }
        }
    }

    // Check for white-on-white (simplified: look for Page Resources with patterns or transparency)
    // Full white-on-white detection needs full graphics state tracking which is complex.
    // We note this as a limitation for now.
    if findings.is_empty() {
        findings.push(HiddenContentFinding {
            page: 0,
            ty: "white_on_white_note".into(),
            description: "White-on-white content detection requires full color-stack tracking. For now, visually inspect the file for white text/shapes on white background.".into(),
            severity: "info".into(),
        });
    }

    findings
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
