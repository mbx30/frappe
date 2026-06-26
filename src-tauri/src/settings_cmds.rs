//! Preferences and settings commands (Issue #241 / #275, #234).
//!
//! Key-value preferences, PDF settings, and alt-text management.

use tauri::State;

use crate::db::Database;
use crate::models::AltTextEntry;

#[tauri::command]
pub fn get_preference(db: State<'_, Database>, key: String) -> Result<Option<String>, String> {
    db.get_preference(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_preference(db: State<'_, Database>, key: String, value: String) -> Result<(), String> {
    db.set_preference(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_preferences(
    db: State<'_, Database>,
) -> Result<std::collections::HashMap<String, String>, String> {
    db.get_all_preferences().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_alt_text(
    db: State<'_, Database>,
    file_path: String,
    object_id: i64,
) -> Result<Option<AltTextEntry>, String> {
    db.get_alt_text(&file_path, object_id)
        .map(|opt| {
            opt.map(|(alt_text, is_decorative)| AltTextEntry {
                object_id,
                alt_text,
                is_decorative,
            })
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_alt_text(
    db: State<'_, Database>,
    file_path: String,
) -> Result<Vec<AltTextEntry>, String> {
    db.get_alt_text_for_file(&file_path)
        .map(|rows| {
            rows.into_iter()
                .map(|(object_id, alt_text, is_decorative)| AltTextEntry {
                    object_id,
                    alt_text,
                    is_decorative,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_alt_text(
    db: State<'_, Database>,
    file_path: String,
    object_id: i64,
    alt_text: String,
    is_decorative: bool,
) -> Result<(), String> {
    db.set_alt_text(&file_path, object_id, &alt_text, is_decorative)
        .map_err(|e| e.to_string())
}
