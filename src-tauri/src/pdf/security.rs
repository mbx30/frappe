use lopdf::{Document, Object};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SecurityFinding {
    pub category: String,
    pub detail: String,
    pub severity: String,
    pub message: String,
}

pub fn check_security(doc: &Document) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();

    // Check for encryption
    let is_encrypted = doc
        .trailer
        .get(b"Encrypt")
        .map(|o| !matches!(o, Object::Null))
        .unwrap_or(false);
    if is_encrypted {
        findings.push(SecurityFinding {
            category: "encryption".into(),
            detail: "Document is encrypted".into(),
            severity: "error".into(),
            message: "PDF is encrypted — password required to process. Encrypted files cannot be preflighted automatically.".into(),
        });
    }

    // Check for JavaScript in catalog. Get the catalog via the trailer's
    // Root reference (not hardcoded (1,0), which fails on most PDFs).
    let catalog_id = match doc.trailer.get(b"Root") {
        Ok(r) => match r.as_reference() {
            Ok(id) => id,
            Err(_) => return findings,
        },
        Err(_) => return findings,
    };
    let catalog = match doc.get_object(catalog_id) {
        Ok(o) => match o.as_dict() {
            Ok(d) => d,
            Err(_) => return findings,
        },
        Err(_) => return findings,
    };

    // Check Names -> JavaScript name tree
    if let Ok(names) = catalog.get(b"Names") {
        if let Ok((_, names_obj)) = doc.dereference(names) {
            if let Ok(names_dict) = names_obj.as_dict() {
                if let Ok(_js) = names_dict.get(b"JavaScript") {
                    findings.push(SecurityFinding {
                        category: "javascript".into(),
                        detail: "JavaScript name tree found".into(),
                        severity: "warning".into(),
                        message: "Document contains JavaScript actions. These may execute when the PDF is opened in a viewer.".into(),
                    });
                }
            }
        }
    }

    // Check for /AA (additional actions) on catalog
    if let Ok(_aa) = catalog.get(b"AA") {
        findings.push(SecurityFinding {
            category: "actions".into(),
            detail: "Additional actions found in catalog".into(),
            severity: "warning".into(),
            message: "Document has additional actions (AA) which may trigger scripts or navigation on open/close.".into(),
        });
    }

    // Check for multimedia annotations page by page
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    for obj_id in &page_ids {
        if let Ok(page_dict) = doc.get_dictionary(*obj_id) {
            if let Ok(annots) = page_dict.get(b"Annots") {
                if let Ok((_, annots_obj)) = doc.dereference(annots) {
                    if let Ok(annots_arr) = annots_obj.as_array() {
                        for annot in annots_arr {
                            if let Ok((_, annot_obj)) = doc.dereference(annot) {
                                if let Ok(annot_dict) = annot_obj.as_dict() {
                                    let subtype = annot_dict
                                        .get(b"Subtype")
                                        .ok()
                                        .and_then(|s| s.as_name().ok());
                                    match subtype {
                                        Some(b"Movie") | Some(b"Sound") | Some(b"Screen")
                                        | Some(b"Widget") => {
                                            let st = String::from_utf8_lossy(subtype.unwrap())
                                                .to_string();
                                            findings.push(SecurityFinding {
                                                category: "multimedia".into(),
                                                detail: format!("Annotation type: /{}", st),
                                                severity: "warning".into(),
                                                message: format!("Page contains a /{} annotation. These may not render correctly in all print workflows.", st),
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    findings
}
