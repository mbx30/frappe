use lopdf::{Document, Object};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct PdfXFinding {
    pub category: String,
    pub detail: String,
    pub severity: String,
    pub message: String,
    pub fix_hint: String,
}

fn obj_to_string(o: &Object) -> String {
    match o {
        Object::Name(n) => String::from_utf8_lossy(n).to_string(),
        Object::String(s, _) => String::from_utf8_lossy(s).to_string(),
        _ => String::new(),
    }
}

fn read_pdf_version_from_header(path: &str) -> String {
    if let Ok(file) = std::fs::File::open(path) {
        use std::io::Read;
        let mut reader = std::io::BufReader::new(file);
        let mut header = [0u8; 100];
        if reader.read(&mut header).is_ok() {
            let s = String::from_utf8_lossy(&header);
            for line in s.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("%PDF-") {
                    return trimmed[5..].trim().to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

/// Parse a PDF version string (e.g. `"1.6"`, `"2.0"`, `"1.7"`) into a f64
/// suitable for comparison against profile minimums.
///
/// Robustness fixes (#171):
///   * Empty / whitespace / `"unknown"` inputs return 0.0 instead of panicking
///     or returning a spurious value from `split`.
///   * Multiple `.` separators (e.g. `"1.6.2"`, `"1.02.3"`) are collapsed to
///     `major.minor` — extra components are ignored, matching how the spec
///     treats sub-minor revisions as cosmetic.
///   * Non-numeric garbage after the version (e.g. `"1.4\n%bin"`,
///     `"1.6abc"`) is stripped rather than failing the whole parse.
///   * Leading non-version text is skipped by scanning for the first digit.
fn parse_version(v: &str) -> f64 {
    let trimmed = v.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
        return 0.0;
    }
    // Find the first ASCII digit — skip any leading noise.
    let start = match trimmed.chars().position(|c| c.is_ascii_digit()) {
        Some(s) => s,
        None => return 0.0,
    };
    let digits_dots: String = trimmed[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if digits_dots.is_empty() {
        return 0.0;
    }
    // Keep at most the first two dot-separated numeric components.
    let parts: Vec<&str> = digits_dots.split('.').collect();
    let major = parts
        .first()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let minor = parts
        .get(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    // Combine into a single f64: 1.6 -> 1.6, 2.0 -> 2.0, 1.10 -> 1.10
    let combined = format!("{major}.{minor}");
    combined.parse::<f64>().unwrap_or(0.0)
}

pub fn check_metadata(doc: &Document) -> Vec<PdfXFinding> {
    let mut findings = Vec::new();

    let info = (|| -> Option<Vec<(Vec<u8>, Object)>> {
        let info_ref = doc.trailer.get(b"Info").ok()?;
        let (_, info_obj) = doc.dereference(info_ref).ok()?;
        let dict = info_obj.as_dict().ok()?;
        Some(dict.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    })();

    let info_dict: std::collections::HashMap<Vec<u8>, Object> =
        info.unwrap_or_default().into_iter().collect();

    let gts_pdfx = info_dict
        .get(&b"GTS_PDFXVersion".to_vec())
        .and_then(|o| doc.dereference(o).ok().map(|(_, o)| obj_to_string(&o)));

    match gts_pdfx {
        Some(ref ver) if !ver.is_empty() => {
            findings.push(PdfXFinding {
                category: "pdfx_version".into(),
                detail: format!("GTS_PDFXVersion: {}", ver),
                severity: "ok".into(),
                message: format!("PDF/X version identifier found: {}", ver),
                fix_hint: String::new(),
            });
        }
        _ => {
            findings.push(PdfXFinding {
                category: "pdfx_version".into(),
                detail: "GTS_PDFXVersion missing".into(),
                severity: "error".into(),
                message: "PDF/X version identifier (GTS_PDFXVersion) not found in document info. This file is not marked as PDF/X.".into(),
                fix_hint: "To add PDF/X metadata: in InDesign, export with PDF/X preset; or use the 'Add OutputIntent' fixup in this app.".into(),
            });
        }
    }

    let trapped = info_dict
        .get(&b"Trapped".to_vec())
        .and_then(|o| doc.dereference(o).ok().map(|(_, o)| obj_to_string(&o)));

    match trapped.as_deref() {
        Some(v @ ("True" | "False" | "Unknown")) => {
            findings.push(PdfXFinding {
                category: "trapped".into(),
                detail: format!("Trapped: /{}", v),
                severity: "ok".into(),
                message: format!("Trapped key present: /{}", v),
                fix_hint: String::new(),
            });
        }
        Some(v) => {
            findings.push(PdfXFinding {
                category: "trapped".into(),
                detail: format!("Trapped: /{}", v),
                severity: "warning".into(),
                message: format!(
                    "Trapped key has unrecognized value: /{}. Expected /True, /False, or /Unknown.",
                    v
                ),
                fix_hint:
                    "Set Trapped to /True, /False, or /Unknown in the document Info dictionary."
                        .into(),
            });
        }
        None => {
            findings.push(PdfXFinding {
                category: "trapped".into(),
                detail: "Trapped key missing".into(),
                severity: "warning".into(),
                message: "Trapped key is missing from document Info. PDF/X requires Trapped to be /True, /False, or /Unknown.".into(),
                fix_hint: "Add 'Trapped' key to the document Info dictionary with value /True, /False, or /Unknown.".into(),
            });
        }
    }

    findings
}

pub fn check_version_compatibility(path: &str, profile: &str) -> Vec<PdfXFinding> {
    let mut findings = Vec::new();
    let version_str = read_pdf_version_from_header(path);
    let version = parse_version(&version_str);

    let (min_version, profile_name) = match profile {
        "x1a" => (1.3, "PDF/X-1a"),
        "x3" => (1.3, "PDF/X-3"),
        "x4" => (1.6, "PDF/X-4"),
        _ => return findings,
    };

    if version == 0.0 {
        findings.push(PdfXFinding {
            category: "pdf_version".into(),
            detail: "Could not determine PDF version".into(),
            severity: "error".into(),
            message: "Unable to read PDF version from file header.".into(),
            fix_hint: "Ensure the file is a valid PDF.".into(),
        });
    } else if version < min_version {
        findings.push(PdfXFinding {
            category: "pdf_version".into(),
            detail: format!("PDF version {} (< {})", version_str, min_version),
            severity: "error".into(),
            message: format!(
                "{} requires PDF version {} or later. This file is version {}.",
                profile_name, min_version, version_str
            ),
            fix_hint: format!(
                "Re-export the PDF with at least PDF {} compatibility. In InDesign: File → Export → PDF → Compatibility → Acrobat {}.",
                min_version,
                if min_version >= 1.6 { "7 (1.6)" } else { "4 (1.3)" }
            ),
        });
    } else {
        findings.push(PdfXFinding {
            category: "pdf_version".into(),
            detail: format!("PDF version {} (≥ {})", version_str, min_version),
            severity: "ok".into(),
            message: format!(
                "PDF version {} meets {} minimum of {}.",
                version_str, profile_name, min_version
            ),
            fix_hint: String::new(),
        });
    }

    findings
}
