//! AI/OCR commands (Phase 5.5, Issue #229).
//!
//! AI visual checking and optical character recognition.

use tauri::State;

use crate::db::Database;
use crate::models::AiCheckResult;
use crate::security;

/// Preference keys for AI daily quota persistence.
const PREFS_KEY_AI_DAY: &str = "ai_daily_usage_date";
const PREFS_KEY_AI_COUNT: &str = "ai_daily_usage_count";

/// Load the persisted daily AI quota from the DB into the in-memory counters
/// so the quota survives app restarts.
fn load_quota_from_db(db: &Database) {
    let day = db
        .get_preference(PREFS_KEY_AI_DAY)
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let count = db
        .get_preference(PREFS_KEY_AI_COUNT)
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    crate::ai_check::load_quota_from_prefs(day, count);
}

/// Persist the current in-memory daily AI quota counters to the DB.
fn save_quota_to_db(db: &Database) {
    let (day, count) = crate::ai_check::quota_snapshot();
    // Silently ignore DB errors — the in-memory counters still enforce the
    // limit for the current session; persistence is best-effort.
    let _ = db.set_preference(PREFS_KEY_AI_DAY, &day.to_string());
    let _ = db.set_preference(PREFS_KEY_AI_COUNT, &count.to_string());
}

#[tauri::command]
pub async fn ai_visual_check(
    db: State<'_, Database>,
    path: String,
    prompt: String,
) -> Result<AiCheckResult, String> {
    let path = security::validate_read_path(&path)?;

    // Restore persisted quota from the DB before the AI call so the
    // in-memory counters reflect usage across app restarts.
    load_quota_from_db(&db);

    let result = crate::ai_check::ai_visual_check(&path.to_string_lossy(), &prompt).await;

    // Persist the updated quota counters back to the DB so the limit
    // survives an app restart.
    save_quota_to_db(&db);

    result
}

// ── OCR commands (Issue #229) ──────────────────────────────────────────
// These are not yet wired into the frontend. Marked #[allow(dead_code)].

/// Detect whether a PDF is text-based or scanned (image-based).
#[allow(dead_code)]
#[tauri::command]
pub fn detect_pdf_type(path: String) -> Result<crate::pdf::ocr::PdfType, String> {
    let pdf_path = security::validate_read_path(&path)?;
    crate::pdf::ocr::detect_pdf_type(&pdf_path)
}

/// Run OCR on a PDF using the specified backend.
#[allow(dead_code)]
#[tauri::command]
pub async fn run_ocr(
    path: String,
    options: crate::pdf::ocr::OcrOptions,
) -> Result<crate::pdf::ocr::OcrResult, String> {
    let pdf_path = security::validate_read_path(&path)?;
    if let Some(ref out) = options.output_path {
        security::validate_write_path(out)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::ocr::run_ocr(&pdf_path, options)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

/// Check if Tesseract OCR engine is available on this system.
#[allow(dead_code)]
#[tauri::command]
pub fn is_tesseract_available() -> Result<bool, String> {
    match crate::pdf::ocr::check_tesseract_available() {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Get the total page count of a PDF.
#[allow(dead_code)]
#[tauri::command]
pub fn get_pdf_page_count(path: String) -> Result<usize, String> {
    let pdf_path = security::validate_read_path(&path)?;
    crate::pdf::ocr::get_page_count(&pdf_path)
}

/// Set Google Cloud Vision API key for cloud-based OCR.
#[allow(dead_code)]
#[tauri::command]
pub fn set_google_vision_api_key(api_key: String) -> Result<(), String> {
    crate::pdf::ocr::set_google_vision_api_key(&api_key)
}

/// Test the Google Cloud Vision API connection with the current API key.
#[allow(dead_code)]
#[tauri::command]
pub async fn test_google_vision_connection() -> Result<bool, String> {
    crate::pdf::ocr::test_google_vision_connection().await
}

/// Estimate the cost of running OCR on a PDF via Google Cloud Vision API.
#[allow(dead_code)]
#[tauri::command]
pub fn estimate_google_vision_cost(path: String) -> Result<crate::pdf::ocr::CostEstimate, String> {
    let pdf_path = security::validate_read_path(&path)?;
    let page_count = crate::pdf::ocr::get_page_count(&pdf_path)?;
    Ok(crate::pdf::ocr::estimate_google_vision_cost(page_count))
}
