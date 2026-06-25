//! PDF/image processing commands.
//!
//! Image replacement/optimization, PDF compression, rendering,
//! page manipulation, layers, content streams, and annotations.

use std::io::{BufReader, Read};

use lopdf::Object;
use tauri::State;

use crate::db::Database;
use crate::models::*;
use crate::pdf::engine::PdfEngine;
use crate::security;

/// Convert a 0-based page index to the 1-based lopdf page ID.
fn lopdf_page_id(page_index: usize) -> u32 {
    (page_index + 1) as u32
}

fn read_pdf_version(path: &str) -> String {
    if let Ok(file) = std::fs::File::open(path) {
        let mut reader = BufReader::new(file);
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

fn get_info_string(lopdf_doc: &lopdf::Document, key: &[u8]) -> String {
    (|| -> Option<String> {
        let info = lopdf_doc.trailer.get(b"Info").ok()?;
        let (_range, info_obj) = lopdf_doc.dereference(info).ok()?;
        let dict = info_obj.as_dict().ok()?;
        let val = dict.get(key).ok()?;
        let (_r, val_obj) = lopdf_doc.dereference(val).ok()?;
        match val_obj {
            Object::String(s, _) => Some(String::from_utf8(s.to_vec()).unwrap_or_default()),
            Object::Name(n) => Some(String::from_utf8(n.to_vec()).unwrap_or_default()),
            _ => None,
        }
    })()
    .unwrap_or_default()
}

// ── PDF open/save metadata ─────────────────────────────────────────────

#[tauri::command]
pub fn open_pdf(engine: State<'_, PdfEngine>, path: String) -> Result<PdfSummary, String> {
    let path_buf = security::validate_read_path(&path)?;
    let file_name = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let doc = engine.open_document(&path)?;
    let page_count = doc.pages().len() as usize;

    let pdf_version = read_pdf_version(&path);

    let lopdf_doc =
        lopdf::Document::load(&path).map_err(|e| format!("Failed to parse PDF metadata: {}", e))?;

    let is_encrypted = lopdf_doc
        .trailer
        .get(b"Encrypt")
        .map(|o| !matches!(o, Object::Null))
        .unwrap_or(false);

    let title = get_info_string(&lopdf_doc, b"Title");
    let creator = get_info_string(&lopdf_doc, b"Creator");
    let producer = get_info_string(&lopdf_doc, b"Producer");
    let creation_date = get_info_string(&lopdf_doc, b"CreationDate");

    Ok(PdfSummary {
        id: 0,
        file_path: path.clone(),
        file_name,
        page_count,
        pdf_version,
        file_size_bytes,
        title,
        creator,
        producer,
        creation_date,
        is_encrypted,
    })
}

#[tauri::command]
pub fn save_pdf_job(db: State<'_, Database>, summary: PdfSummary) -> Result<i64, String> {
    db.save_pdf_job(&summary).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_pdf_jobs(db: State<'_, Database>) -> Result<Vec<PdfSummary>, String> {
    db.list_pdf_jobs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_pdf_job(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_pdf_job(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_certified_version(
    db: State<'_, Database>,
    job_id: i64,
    file_path: String,
    author: String,
    comment: String,
) -> Result<i64, String> {
    let file_path = security::validate_read_path(&file_path)?;
    let file_path_str = file_path.to_str().ok_or("file path is not valid UTF-8")?;
    let metadata = std::fs::metadata(&file_path).map_err(|e| format!("File not found: {}", e))?;
    db.save_certified_version(job_id, file_path_str, metadata.len(), &author, &comment)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_certified_versions(
    db: State<'_, Database>,
    job_id: i64,
) -> Result<Vec<CertifiedVersion>, String> {
    db.list_certified_versions(job_id)
        .map_err(|e| e.to_string())
}

// ── PDF Rendering ──────────────────────────────────────────────────────

#[tauri::command]
pub fn render_page_thumbnail(
    engine: State<'_, PdfEngine>,
    path: String,
    page_index: usize,
    width_px: Option<u32>,
) -> Result<String, String> {
    let path = security::validate_read_path(&path)?;
    let path_str = path.to_str().ok_or("path is not valid UTF-8")?;
    use image::RgbaImage;
    let doc = engine.open_document(path_str)?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let width: i32 = width_px.unwrap_or(120) as i32;
    let config = pdfium_render::prelude::PdfRenderConfig::new().set_target_width(width);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("frappe_pdf");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Temp dir error: {}", e))?;
    let out_path = temp_dir.join(format!(
        "thumb_{page_index}_{}_{}.png",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    if bytes.len() < (pw as usize) * (ph as usize) * 4 {
        return Err("Rendered bitmap is shorter than expected".to_string());
    }
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(
                x,
                y,
                image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]),
            );
        }
    }
    img.save(&out_path)
        .map_err(|e| format!("Save error: {}", e))?;
    Ok(out_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn render_page(
    engine: State<'_, PdfEngine>,
    path: String,
    page_index: usize,
    dpi: Option<f32>,
) -> Result<String, String> {
    let path = security::validate_read_path(&path)?;
    let path_str = path.to_str().ok_or("path is not valid UTF-8")?;
    use image::RgbaImage;
    use pdfium_render::prelude::PdfRenderConfig;
    let doc = engine.open_document(path_str)?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let dpi_val = dpi.unwrap_or(144.0) as f64;
    let page_width = page.width().value as f64;
    let px_width = (page_width * dpi_val / 72.0) as i32;
    let config = PdfRenderConfig::new().set_target_width(px_width);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("frappe_pdf");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Temp dir error: {}", e))?;
    let out_path = temp_dir.join(format!(
        "page_{page_index}_{}_{}.png",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    if bytes.len() < (pw as usize) * (ph as usize) * 4 {
        return Err("Rendered bitmap is shorter than expected".to_string());
    }
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(
                x,
                y,
                image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]),
            );
        }
    }
    img.save(&out_path)
        .map_err(|e| format!("Save error: {}", e))?;
    Ok(out_path.to_string_lossy().to_string())
}

#[derive(serde::Serialize)]
pub struct PageDimensions {
    pub width_pts: f64,
    pub height_pts: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

#[tauri::command]
pub fn render_page_with_overprint(
    engine: State<'_, PdfEngine>,
    path: String,
    page_index: usize,
    dpi: Option<f32>,
) -> Result<String, String> {
    let path = security::validate_read_path(&path)?;
    let path_str = path.to_str().ok_or("path is not valid UTF-8")?;
    use image::RgbaImage;
    use pdfium_render::prelude::PdfRenderConfig;
    let doc = engine.open_document(path_str)?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let dpi_val = dpi.unwrap_or(144.0) as f64;
    let page_width = page.width().value as f64;
    let px_width = (page_width * dpi_val / 72.0) as i32;
    let config = PdfRenderConfig::new()
        .set_target_width(px_width)
        .use_print_quality(true);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("frappe_pdf");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Temp dir error: {}", e))?;
    let out_path = temp_dir.join(format!(
        "page_{page_index}_overprint_{}_{}.png",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    if bytes.len() < (pw as usize) * (ph as usize) * 4 {
        return Err("Rendered bitmap is shorter than expected".to_string());
    }
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(
                x,
                y,
                image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]),
            );
        }
    }
    img.save(&out_path)
        .map_err(|e| format!("Save error: {}", e))?;
    Ok(out_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_page_dimensions(
    engine: State<'_, PdfEngine>,
    path: String,
    page_index: usize,
) -> Result<PageDimensions, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = engine.open_document(_path.to_str().ok_or("path is not valid UTF-8")?)?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let w = page.width().value as f64;
    let h = page.height().value as f64;
    Ok(PageDimensions {
        width_pts: w,
        height_pts: h,
        width_mm: w * 0.3528,
        height_mm: h * 0.3528,
    })
}

// ── Page operations ────────────────────────────────────────────────────

#[tauri::command]
pub fn extract_pages(path: String, indices: Vec<usize>, output_path: String) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let pages = doc.get_pages();
    let all_page_numbers: Vec<u32> = pages.keys().copied().collect();
    let to_keep: std::collections::HashSet<u32> = indices
        .iter()
        .filter_map(|i| all_page_numbers.get(*i))
        .copied()
        .collect();
    let to_remove: Vec<u32> = all_page_numbers
        .iter()
        .filter(|pn| !to_keep.contains(pn))
        .copied()
        .collect();
    doc.delete_pages(&to_remove);
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn delete_pages(path: String, indices: Vec<usize>, output_path: String) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let pages = doc.get_pages();
    let all_page_numbers: Vec<u32> = pages.keys().copied().collect();
    let to_remove: Vec<u32> = indices
        .iter()
        .filter_map(|i| all_page_numbers.get(*i))
        .copied()
        .collect();
    doc.delete_pages(&to_remove);
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn rotate_page(
    path: String,
    page_index: usize,
    degrees: i64,
    output_path: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let pages = doc.get_pages();
    let obj_id = match pages.get(&lopdf_page_id(page_index)) {
        Some(id) => *id,
        None => return Err(format!("Page {} not found", page_index)),
    };
    if let Ok(page) = doc.get_dictionary_mut(obj_id) {
        page.set("Rotate", Object::Integer(degrees));
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_pdf_catalog(path: String) -> Result<serde_json::Value, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let root_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "Failed to find Root reference in trailer".to_string())?;
    let catalog = doc
        .get_object(root_ref)
        .map_err(|e| format!("Failed to get catalog: {}", e))?;
    let dict = catalog
        .as_dict()
        .map_err(|_| "Catalog is not a dictionary".to_string())?;

    let mut result = serde_json::Map::new();
    for (key, value) in dict.iter() {
        let key_str = String::from_utf8_lossy(key).to_string();
        let val_str = match value {
            Object::Name(n) => format!("/{}", String::from_utf8_lossy(n)),
            Object::Integer(i) => i.to_string(),
            Object::Real(r) => r.to_string(),
            Object::String(s, _) => String::from_utf8_lossy(s).to_string(),
            Object::Array(a) => format!("[{} elements]", a.len()),
            Object::Dictionary(d) => format!("dict ({} entries)", d.len()),
            Object::Reference(r) => format!("{} {} R", r.0, r.1),
            Object::Stream(_) => "stream".to_string(),
            Object::Null => "null".to_string(),
            Object::Boolean(b) => b.to_string(),
        };
        result.insert(key_str, serde_json::Value::String(val_str));
    }

    let page_count = doc.get_pages().len();
    result.insert(
        "PageCount".to_string(),
        serde_json::Value::Number(serde_json::Number::from(page_count as u64)),
    );

    let pdf_version = {
        let path_buf = std::path::PathBuf::from(&path);
        let mut header = [0u8; 100];
        if let Ok(file) = std::fs::File::open(&path_buf) {
            use std::io::Read;
            let mut reader = std::io::BufReader::new(file);
            if reader.read(&mut header).is_ok() {
                String::from_utf8_lossy(&header)
                    .lines()
                    .next()
                    .and_then(|l| {
                        if l.trim().starts_with("%PDF-") {
                            Some(l.trim()[5..].to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        }
    };
    result.insert(
        "PDFVersion".to_string(),
        serde_json::Value::String(pdf_version),
    );

    Ok(serde_json::Value::Object(result))
}

// ── Layers & page operations (Phase 3.2) ──────────────────────────────

#[tauri::command]
pub fn reorder_pages(
    path: String,
    new_order: Vec<usize>,
    output_path: String,
) -> Result<(), String> {
    use lopdf::Object;
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let pages = doc.get_pages();
    let all_page_numbers: Vec<u32> = pages.keys().copied().collect();
    if new_order.len() != all_page_numbers.len() {
        return Err(format!(
            "New order length ({}) does not match page count ({})",
            new_order.len(),
            all_page_numbers.len()
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for &idx in &new_order {
        if !seen.insert(idx) {
            return Err(format!("Duplicate page index {idx} in new_order"));
        }
    }
    let root_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "No Root reference".to_string())?;
    let catalog = doc
        .get_dictionary(root_ref)
        .map_err(|e| format!("Catalog error: {e}"))?;
    let pages_ref = catalog
        .get(b"Pages")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "No Pages reference".to_string())?;
    let mut new_kids: Vec<Object> = Vec::new();
    for idx in &new_order {
        if let Some(obj_ref) = pages.get(&lopdf_page_id(*idx)) {
            new_kids.push(Object::Reference(*obj_ref));
        } else {
            return Err(format!("Page index {idx} out of range"));
        }
    }
    if let Ok(pages_dict) = doc.get_dictionary_mut(pages_ref) {
        pages_dict.set("Kids", Object::Array(new_kids));
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn insert_blank_page(
    path: String,
    after_index: usize,
    width_mm: f64,
    height_mm: f64,
    output_path: String,
) -> Result<(), String> {
    use lopdf::Object;
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let width_pts = width_mm / 0.3528;
    let height_pts = height_mm / 0.3528;
    let root_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "Cannot find Root".to_string())?;
    let catalog = doc
        .get_dictionary(root_ref)
        .map_err(|e| format!("Catalog: {e}"))?;
    let pages_ref = catalog
        .get(b"Pages")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "Cannot find Pages ref".to_string())?;
    let media_box = Object::Array(vec![
        Object::Real(0.0),
        Object::Real(0.0),
        Object::Real(width_pts as f32),
        Object::Real(height_pts as f32),
    ]);
    let page_id = doc.new_object_id();
    let page_dict = lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
        (b"Parent".to_vec(), Object::Reference(pages_ref)),
        (b"MediaBox".to_vec(), media_box),
        (
            b"Resources".to_vec(),
            Object::Dictionary(lopdf::Dictionary::new()),
        ),
    ]);
    doc.objects.insert(page_id, Object::Dictionary(page_dict));

    let pages = doc.get_pages();
    let page_refs: Vec<(u32, u16)> = pages.values().copied().collect();
    let insert_pos = (after_index + 1).min(page_refs.len());
    let original_count = doc.get_pages().len();
    if let Ok(pages_dict) = doc.get_dictionary_mut(pages_ref) {
        if let Ok(kids) = pages_dict.get(b"Kids") {
            if let Object::Array(arr) = kids {
                let mut new_kids = arr.clone();
                let new_ref = Object::Reference(page_id);
                if insert_pos >= new_kids.len() {
                    new_kids.push(new_ref);
                } else {
                    new_kids.insert(insert_pos, new_ref);
                }
                pages_dict.set("Kids", Object::Array(new_kids));
                pages_dict.set("Count", Object::Integer((original_count + 1) as i64));
            }
        }
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn list_layers(path: String) -> Result<Vec<LayerInfo>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let mut layers = Vec::new();
    for (obj_id, obj) in &doc.objects {
        if let lopdf::Object::Dictionary(dict) = obj {
            let type_val = dict.get(b"Type").ok();
            let is_ocg = match type_val {
                Some(lopdf::Object::Name(n)) => n == b"OCG",
                _ => false,
            };
            if is_ocg {
                let name = match dict.get(b"Name").ok() {
                    Some(lopdf::Object::String(s, _)) => String::from_utf8_lossy(s).to_string(),
                    Some(lopdf::Object::Name(n)) => String::from_utf8_lossy(n).to_string(),
                    _ => String::new(),
                };
                let visible = dict.get(b"OC").ok().map(|o| !matches!(o, lopdf::Object::Name(n) if n == b"OFF")).unwrap_or(true);
                layers.push(LayerInfo {
                    name,
                    visible,
                    locked: false,
                    object_id: obj_id.0,
                });
            }
        }
    }
    Ok(layers)
}

#[tauri::command]
pub fn set_layer_visibility(
    path: String,
    object_id: u32,
    visible: bool,
    output_path: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    use lopdf::Object;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let key = (object_id, 0u16);
    let target = doc
        .objects
        .get_mut(&key)
        .ok_or_else(|| format!("OCG object {object_id} not found"))?;
    if let Object::Dictionary(d) = target {
        if visible {
            d.remove(b"OC");
        } else {
            d.set("OC", Object::Name(b"OFF".to_vec()));
        }
    } else {
        return Err(format!("OCG {object_id} is not a dictionary"));
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(())
}

// ── Content-stream operations (Phase 3.3) ──────────────────────────────

#[tauri::command]
pub fn decode_content_stream(path: String, page_index: usize) -> Result<String, String> {
    use crate::pdf::content_stream;
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let pages = doc.get_pages();
    let obj_id = pages
        .get(&lopdf_page_id(page_index))
        .copied()
        .ok_or_else(|| format!("Page {page_index} not found"))?;
    let page_dict = doc
        .get_dictionary(obj_id)
        .map_err(|e| format!("Page dict error: {e}"))?;
    let contents = page_dict
        .get(b"Contents")
        .map_err(|_| "No Contents key".to_string())?;
    match contents {
        lopdf::Object::Stream(stream) => {
            let decoded = content_stream::decode_stream(stream)?;
            Ok(String::from_utf8_lossy(&decoded).to_string())
        }
        lopdf::Object::Reference(contents_ref) => {
            if let Ok(stream_obj) = doc.get_object(*contents_ref) {
                if let lopdf::Object::Stream(stream) = stream_obj {
                    let decoded = content_stream::decode_stream(stream)?;
                    Ok(String::from_utf8_lossy(&decoded).to_string())
                } else {
                    Err("Contents is not a stream".to_string())
                }
            } else {
                Err("Cannot resolve Contents reference".to_string())
            }
        }
        lopdf::Object::Array(arr) => {
            let mut combined = Vec::new();
            for elem in arr {
                let stream_obj = match elem {
                    lopdf::Object::Reference(r) => match doc.get_object(*r) {
                        Ok(o) => o,
                        Err(_) => continue,
                    },
                    lopdf::Object::Stream(_) => elem,
                    _ => continue,
                };
                if let lopdf::Object::Stream(stream) = stream_obj {
                    let decoded = content_stream::decode_stream(stream)?;
                    if !combined.is_empty() {
                        combined.push(b'\n');
                    }
                    combined.extend_from_slice(&decoded);
                }
            }
            if combined.is_empty() {
                return Err("Contents array contained no decodable streams".to_string());
            }
            Ok(String::from_utf8_lossy(&combined).to_string())
        }
        _ => Err(format!(
            "Unexpected Contents type: {:?}",
            contents.type_name()
        )),
    }
}

#[tauri::command]
pub fn encode_content_stream(
    path: String,
    page_index: usize,
    content: String,
    output_path: String,
) -> Result<(), String> {
    use crate::pdf::content_stream;
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let pages = doc.get_pages();
    let obj_id = pages
        .get(&lopdf_page_id(page_index))
        .copied()
        .ok_or_else(|| format!("Page {page_index} not found"))?;
    let stream = content_stream::encode_stream(content.as_bytes());
    let stream_id = doc.add_object(lopdf::Object::Stream(stream));
    if let Ok(page_dict) = doc.get_dictionary_mut(obj_id) {
        page_dict.set("Contents", lopdf::Object::Reference(stream_id));
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn round_trip_page(
    path: String,
    page_index: usize,
    output_path: String,
) -> Result<serde_json::Value, String> {
    let decoded = decode_content_stream(path.clone(), page_index)?;
    encode_content_stream(path, page_index, decoded.clone(), output_path)?;
    Ok(serde_json::json!({
        "page_index": page_index,
        "decoded_bytes": decoded.len(),
        "success": true,
    }))
}

#[tauri::command]
pub fn tokenize_content_stream(path: String, page_index: usize) -> Result<Vec<String>, String> {
    use crate::pdf::content_stream;
    let content = decode_content_stream(path, page_index)?;
    let tokens = content_stream::tokenize_content(content.as_bytes());
    Ok(tokens.iter().map(|t| format!("{t:?}")).collect())
}

// ── Image replacement & optimization (Phase 3.5) ───────────────────────

#[tauri::command]
pub fn replace_image(
    path: String,
    _page_index: usize,
    xobject_name: String,
    new_image_path: String,
    output_path: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let new_image_path = security::validate_read_path(&new_image_path)?;
    let output_path = security::validate_write_path(&output_path)?;

    use lopdf::Object;
    use std::io::Cursor;

    let replacement_bytes = std::fs::read(&new_image_path)
        .map_err(|e| format!("read replacement image: {e}"))?;
    let format = image::guess_format(&replacement_bytes)
        .map_err(|e| format!("detect image format: {e}"))?;
    let dyn_img = image::load_from_memory(&replacement_bytes)
        .map_err(|e| format!("decode replacement image: {e}"))?;
    let width = dyn_img.width();
    let height = dyn_img.height();
    if width == 0 || height == 0 {
        return Err("Replacement image has zero dimension".to_string());
    }

    let has_alpha = matches!(dyn_img, image::DynamicImage::ImageRgba8(_));
    let is_palette = format == image::ImageFormat::Png;
    let (encoded_bytes, filter_name, color_space) = if has_alpha && is_palette {
        let mut out = Vec::new();
        dyn_img
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| format!("encode PNG: {e}"))?;
        (out, b"FlateDecode".to_vec(), b"DeviceRGB".to_vec())
    } else {
        let rgb = dyn_img.to_rgb8();
        let mut out = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
        use image::ImageEncoder;
        encoder
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ColorType::Rgb8.into(),
            )
            .map_err(|e| format!("encode JPEG: {e}"))?;
        (out, b"DCTDecode".to_vec(), b"DeviceRGB".to_vec())
    };

    let mut doc = lopdf::Document::load(&path)
        .map_err(|e| format!("Failed to open PDF: {e}"))?;
    let name_bytes = xobject_name.as_bytes().to_vec();

    let page_id = {
        let pages = doc.get_pages();
        if pages.is_empty() {
            return Err("PDF has no pages".to_string());
        }
        let page_num = (lopdf_page_id(_page_index) as u32).max(1);
        *pages
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", _page_index))?
    };
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|e| format!("page dict: {e}"))?;
    let resources = page_dict
        .get(b"Resources")
        .ok()
        .and_then(|o| match o {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
            _ => None,
        })
        .ok_or_else(|| "no Resources on page".to_string())?;
    let xobject_dict = resources
        .get(b"XObject")
        .ok()
        .and_then(|o| match o {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
            _ => None,
        })
        .ok_or_else(|| "no XObject dict on page".to_string())?;

    let target_id = if xobject_name.is_empty() {
        let mut found = None;
        for (_k, v) in xobject_dict.iter() {
            if let Object::Reference(r) = v {
                if let Ok(obj) = doc.get_object(*r) {
                    if let Ok(stream) = obj.as_stream() {
                        if let Ok(Object::Name(n)) = stream.dict.get(b"Subtype") {
                            if n == b"Image" {
                                found = Some(*r);
                                break;
                            }
                        }
                    }
                }
            }
        }
        found.ok_or_else(|| "no Image XObject on page".to_string())?
    } else {
        let v = xobject_dict
            .get(&name_bytes)
            .map_err(|e| format!("get xobject: {e}"))?;
        match v {
            Object::Reference(r) => *r,
            _ => return Err("XObject is not a reference".to_string()),
        }
    };

    if let Some(obj) = doc.objects.get_mut(&target_id) {
        if let Ok(stream) = obj.as_stream_mut() {
            stream.content = encoded_bytes;
            stream.dict.set("Filter", Object::Name(filter_name));
            stream.dict.set("ColorSpace", Object::Name(color_space));
            stream.dict.set("Width", Object::Integer(width as i64));
            stream.dict.set("Height", Object::Integer(height as i64));
            stream.dict.set("BitsPerComponent", Object::Integer(8));
            stream.dict.remove(b"Length");
            stream.dict.remove(b"DecodeParms");
        } else {
            return Err("Target XObject is not a stream".to_string());
        }
    } else {
        return Err(format!("XObject {} not found in document", target_id.0));
    }

    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn optimize_image(
    path: String,
    page_index: usize,
    xobject_name: String,
    settings: OptimizeSettings,
    output_path: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;

    use lopdf::Object;

    let quality = settings.quality.unwrap_or(85).clamp(1, 100);
    let max_w = settings.max_width.unwrap_or(0);
    let max_h = settings.max_height.unwrap_or(0);
    let force_jpeg = settings.convert_to_jpeg.unwrap_or(true);

    let mut doc = lopdf::Document::load(&path)
        .map_err(|e| format!("Failed to open PDF: {e}"))?;
    let page_id = {
        let pages = doc.get_pages();
        if pages.is_empty() {
            return Err("PDF has no pages".to_string());
        }
        let page_num = lopdf_page_id(page_index) as u32;
        *pages
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_index))?
    };
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|e| format!("page dict: {e}"))?;
    let resources = page_dict
        .get(b"Resources")
        .ok()
        .and_then(|o| match o {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
            _ => None,
        })
        .ok_or_else(|| "no Resources on page".to_string())?;
    let xobject_dict = resources
        .get(b"XObject")
        .ok()
        .and_then(|o| match o {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
            _ => None,
        })
        .ok_or_else(|| "no XObject dict on page".to_string())?;

    let target_id = if xobject_name.is_empty() {
        let mut found = None;
        for (_k, v) in xobject_dict.iter() {
            if let Object::Reference(r) = v {
                if let Ok(obj) = doc.get_object(*r) {
                    if let Ok(stream) = obj.as_stream() {
                        if let Ok(Object::Name(n)) = stream.dict.get(b"Subtype") {
                            if n == b"Image" {
                                found = Some(*r);
                                break;
                            }
                        }
                    }
                }
            }
        }
        found.ok_or_else(|| "no Image XObject on page".to_string())?
    } else {
        let v = xobject_dict
            .get(xobject_name.as_bytes())
            .map_err(|e| format!("get xobject: {e}"))?;
        match v {
            Object::Reference(r) => *r,
            _ => return Err("XObject is not a reference".to_string()),
        }
    };

    let (orig_w, orig_h, orig_bpc, orig_cs) = {
        let stream = doc
            .get_object(target_id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .ok_or_else(|| "target not a stream".to_string())?;
        let w = stream
            .dict
            .get(b"Width")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0) as u32;
        let h = stream
            .dict
            .get(b"Height")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0) as u32;
        let bpc = stream
            .dict
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(8) as u32;
        let cs = stream
            .dict
            .get(b"ColorSpace")
            .ok()
            .and_then(|o| match o {
                Object::Name(n) => Some(n.clone()),
                _ => None,
            })
            .unwrap_or_else(|| b"DeviceRGB".to_vec());
        (w, h, bpc, cs)
    };
    if orig_w == 0 || orig_h == 0 {
        return Err("Image has zero dimension".to_string());
    }

    let stream = doc
        .get_object(target_id)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .ok_or_else(|| "target not a stream".to_string())?;
    let raw = stream.content.clone();

    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let decompressed = if stream
        .dict
        .get(b"Filter")
        .ok()
        .map(|o| {
            matches!(o, Object::Name(n) if n == b"FlateDecode" || n == b"Fl")
        })
        .unwrap_or(false)
    {
        let mut d = ZlibDecoder::new(raw.as_slice());
        let mut out = Vec::new();
        d.read_to_end(&mut out)
            .map_err(|e| format!("decompress: {e}"))?;
        out
    } else {
        raw
    };

    let channels: u32 = match orig_cs.as_slice() {
        b"DeviceGray" | b"G" => 1,
        b"DeviceRGB" | b"RGB" => 3,
        b"DeviceCMYK" | b"CMYK" => 4,
        _ => 3,
    };
    let bpp = (channels * orig_bpc / 8) as usize;
    let expected = (orig_w as usize) * (orig_h as usize) * bpp;
    if decompressed.len() < expected {
        return Err(format!(
            "image data too short: have {} need {}",
            decompressed.len(),
            expected
        ));
    }
    let _color = match channels {
        1 => image::ColorType::L8,
        3 => image::ColorType::Rgb8,
        4 => image::ColorType::Rgba8,
        _ => return Err("unsupported channel count".to_string()),
    };
    let img = match channels {
        3 => {
            let buf: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(orig_w, orig_h, decompressed)
                    .ok_or_else(|| "rgb raw".to_string())?;
            image::DynamicImage::ImageRgb8(buf)
        }
        1 => {
            let buf: image::ImageBuffer<image::Luma<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(orig_w, orig_h, decompressed)
                    .ok_or_else(|| "gray raw".to_string())?;
            use image::buffer::ConvertBuffer;
            let rgb: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> = buf.convert();
            image::DynamicImage::ImageRgb8(rgb)
        }
        4 => {
            let buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(orig_w, orig_h, decompressed)
                    .ok_or_else(|| "rgba raw".to_string())?;
            image::DynamicImage::ImageRgba8(buf)
        }
        _ => return Err("unsupported channel count".to_string()),
    };

    let mut target_w = orig_w;
    let mut target_h = orig_h;
    if max_w > 0 && max_w < orig_w {
        let scale = max_w as f32 / orig_w as f32;
        target_w = max_w;
        target_h = ((orig_h as f32) * scale).round().max(1.0) as u32;
    }
    if max_h > 0 && max_h < target_h {
        let scale = max_h as f32 / target_h as f32;
        target_h = max_h;
        target_w = ((target_w as f32) * scale).round().max(1.0) as u32;
    }
    let final_img = if target_w != orig_w || target_h != orig_h {
        img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let (new_bytes, filter, cs_name): (Vec<u8>, &[u8], &[u8]) = if force_jpeg {
        let rgb = final_img.to_rgb8();
        let mut out = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        use image::ImageEncoder;
        encoder
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ColorType::Rgb8.into(),
            )
            .map_err(|e| format!("encode JPEG: {e}"))?;
        (out, b"DCTDecode", b"DeviceRGB")
    } else {
        let gray = final_img.to_luma8();
        let mut out = Vec::new();
        let encoder =
            image::codecs::png::PngEncoder::new(&mut out);
        use image::ImageEncoder;
        encoder
            .write_image(
                gray.as_raw(),
                gray.width(),
                gray.height(),
                image::ColorType::L8.into(),
            )
            .map_err(|e| format!("encode PNG: {e}"))?;
        (out, b"FlateDecode", b"DeviceGray")
    };

    if let Some(obj) = doc.objects.get_mut(&target_id) {
        if let Ok(stream_obj) = obj.as_stream_mut() {
            stream_obj.content = new_bytes;
            stream_obj.dict.set("Filter", Object::Name(filter.to_vec()));
            stream_obj.dict.set("ColorSpace", Object::Name(cs_name.to_vec()));
            stream_obj.dict.set("Width", Object::Integer(target_w as i64));
            stream_obj.dict.set("Height", Object::Integer(target_h as i64));
            stream_obj.dict.set("BitsPerComponent", Object::Integer(8));
            stream_obj.dict.remove(b"Length");
            stream_obj.dict.remove(b"DecodeParms");
        } else {
            return Err("Target XObject is not a stream".to_string());
        }
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save: {e}"))?;
    Ok(())
}

// ── PDF Annotations (#230) ─────────────────────────────────────────────

#[tauri::command]
pub fn pdf_annotation_add(
    db: State<'_, Database>,
    file_path: String,
    page: i64,
    annotation_type: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: String,
    content: String,
) -> Result<crate::models::PdfAnnotation, String> {
    if !crate::pdf::annotations::is_valid_rect(width, height) {
        return Err("annotation must have positive width and height".into());
    }
    db.add_annotation(&file_path, page, &annotation_type, x, y, width, height, &color, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pdf_annotations_list(
    db: State<'_, Database>,
    file_path: String,
) -> Result<Vec<crate::models::PdfAnnotation>, String> {
    db.list_annotations(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pdf_annotation_update(
    db: State<'_, Database>,
    id: i64,
    color: Option<String>,
    content: Option<String>,
) -> Result<crate::models::PdfAnnotation, String> {
    db.update_annotation(id, color.as_deref(), content.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pdf_annotation_delete(
    db: State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    db.delete_annotation(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pdf_annotation_page_counts(
    db: State<'_, Database>,
    file_path: String,
) -> Result<std::collections::HashMap<i64, i64>, String> {
    db.annotation_page_counts(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pdf_annotation_reply_add(
    db: State<'_, Database>,
    annotation_id: i64,
    content: String,
) -> Result<crate::models::PdfAnnotationReply, String> {
    db.add_annotation_reply(annotation_id, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pdf_annotation_replies_list(
    db: State<'_, Database>,
    annotation_id: i64,
) -> Result<Vec<crate::models::PdfAnnotationReply>, String> {
    db.list_annotation_replies(annotation_id)
        .map_err(|e| e.to_string())
}
