use lopdf::{Dictionary, Document, Object};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct FontFinding {
    pub font_name: String,
    pub font_type: String,
    pub is_embedded: bool,
    pub is_subsetted: bool,
    pub pages: Vec<usize>,
    pub severity: String,
    pub message: String,
}

fn detect_subsetting(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() > 7 && bytes[6] == b'+' && bytes[..6].iter().all(|b| b.is_ascii_uppercase())
}

fn obj_to_string(o: &Object) -> String {
    match o {
        Object::Name(n) => String::from_utf8_lossy(n).to_string(),
        Object::String(s, _) => String::from_utf8_lossy(s).to_string(),
        _ => String::new(),
    }
}

fn dict_get_string(dict: &Dictionary, doc: &Document, key: &[u8]) -> String {
    dict.get(key)
        .ok()
        .and_then(|o| doc.dereference(o).ok())
        .map(|(_, o)| obj_to_string(o))
        .unwrap_or_default()
}

fn check_font_embedded(dict: &Dictionary, doc: &Document) -> bool {
    dict.get(b"FontDescriptor")
        .ok()
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, fd)| fd.as_dict().ok())
        .map(|fd| {
            fd.get(b"FontFile").is_ok()
                || fd.get(b"FontFile2").is_ok()
                || fd.get(b"FontFile3").is_ok()
        })
        .unwrap_or(false)
}

fn collect_fonts_from_dict(
    dict: &Dictionary,
    doc: &Document,
    fonts: &mut Vec<(String, String, bool)>,
) {
    for (_key, value) in dict.iter() {
        if let Ok((_, obj)) = doc.dereference(value) {
            if let Ok(font_dict) = obj.as_dict() {
                let name = dict_get_string(font_dict, doc, b"BaseFont");
                if name.is_empty() {
                    continue;
                }
                let font_type = dict_get_string(font_dict, doc, b"Subtype");
                let is_embedded = check_font_embedded(font_dict, doc);
                fonts.push((name.clone(), font_type.clone(), is_embedded));

                // Check DescendantFonts for Type0/CIDFont
                if let Ok(desc_ref) = font_dict.get(b"DescendantFonts") {
                    if let Ok((_, arr_obj)) = doc.dereference(desc_ref) {
                        if let Ok(arr) = arr_obj.as_array() {
                            for item in arr {
                                if let Ok((_, item_obj)) = doc.dereference(item) {
                                    if let Ok(cid_dict) = item_obj.as_dict() {
                                        let cid_name = dict_get_string(cid_dict, doc, b"BaseFont");
                                        if !cid_name.is_empty() {
                                            let cid_embedded = check_font_embedded(cid_dict, doc);
                                            fonts.push((cid_name, font_type.clone(), cid_embedded));
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

fn walk_resources(dict: &Dictionary, doc: &Document, fonts: &mut Vec<(String, String, bool)>) {
    if let Ok(res_ref) = dict.get(b"Resources") {
        if let Ok((_, res_obj)) = doc.dereference(res_ref) {
            if let Ok(res_dict) = res_obj.as_dict() {
                if let Ok(font_ref) = res_dict.get(b"Font") {
                    if let Ok((_, font_obj)) = doc.dereference(font_ref) {
                        if let Ok(fd) = font_obj.as_dict() {
                            collect_fonts_from_dict(fd, doc, fonts);
                        }
                    }
                }
            }
        }
    }
}

pub fn collect_fonts(doc: &Document) -> Vec<FontFinding> {
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();

    let mut seen: std::collections::HashMap<String, FontFinding> = std::collections::HashMap::new();
    let mut page_sets: std::collections::HashMap<String, std::collections::HashSet<usize>> =
        std::collections::HashMap::new();
    for page_num in 0..page_ids.len() {
        let obj_id = page_ids[page_num];
        if let Ok(page_dict) = doc.get_dictionary(obj_id) {
            let mut page_fonts = Vec::new();
            walk_resources(page_dict, doc, &mut page_fonts);
            for (name, font_type, is_embedded) in page_fonts {
                seen.entry(name.clone()).or_insert_with(|| {
                    let is_subsetted = detect_subsetting(&name);
                    let (severity, message) = if !is_embedded {
                        (
                            "error".to_string(),
                            format!("Font '{}' is not embedded. The receiving printer may substitute a different font.", name),
                        )
                    } else if !is_subsetted {
                        (
                            "warning".to_string(),
                            format!("Font '{}' is fully embedded (not subsetted). Consider subsetting to reduce file size.", name),
                        )
                    } else {
                        ("ok".to_string(), format!("Font '{}' is embedded and subsetted.", name))
                    };
                    FontFinding {
                        font_name: name.clone(),
                        font_type,
                        is_embedded,
                        is_subsetted,
                        pages: Vec::new(),
                        severity,
                        message,
                    }
                });
                page_sets.entry(name).or_default().insert(page_num + 1);
            }
        }
    }

    // Merge deduplicated page sets into the findings.
    for (name, set) in page_sets {
        if let Some(finding) = seen.get_mut(&name) {
            let mut pages: Vec<usize> = set.into_iter().collect();
            pages.sort();
            finding.pages = pages;
        }
    }

    let mut result: Vec<FontFinding> = seen.into_values().collect();
    result.sort_by(|a, b| {
        let sev_order = |s: &str| match s {
            "error" => 0,
            "warning" => 1,
            "ok" => 2,
            _ => 3,
        };
        sev_order(&a.severity).cmp(&sev_order(&b.severity))
    });
    result
}
