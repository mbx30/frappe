//! Database administration commands (#90, #99).
//!
//! Schema version, integrity verification, backup/restore.

use std::path::PathBuf;
use tauri::State;

use crate::db::Database;
use crate::security;

#[tauri::command]
pub fn verify_database(db: State<'_, Database>) -> crate::db::VerificationResult {
    db.verify_integrity()
}

#[tauri::command]
pub fn get_schema_version(db: State<'_, Database>) -> Result<i64, String> {
    db.get_schema_version().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_backup(
    db: State<'_, Database>,
    backup_path: String,
) -> Result<crate::models::BackupEntry, String> {
    let path = security::validate_write_path(&backup_path)?;
    db.create_backup(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_backups(db: State<'_, Database>) -> Result<Vec<crate::models::BackupEntry>, String> {
    db.list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_plaintext_backup(
    db: State<'_, Database>,
    output_path: String,
) -> Result<u64, String> {
    let output_path = security::validate_write_path(&output_path)?;
    let path = PathBuf::from(&output_path);
    db.export_plaintext_backup(&path).map_err(|e| e.to_string())
}
