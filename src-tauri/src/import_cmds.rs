//! Import commands (Phase 1.3).
//!
//! Commands for importing data from CSV, Excel, Google Sheets, and Notion.

use tauri::State;

use crate::cloud_import;
use crate::db::Database;
use crate::models::{ImportResult, SheetData};
use crate::security;

#[tauri::command]
pub fn import_csv_file(
    db: State<'_, Database>,
    workbook_id: i64,
    file_path: String,
) -> Result<SheetData, String> {
    let path = security::validate_read_path(&file_path)?;
    let (sheet_name, headers, rows) = crate::import::import_csv_data(&path)?;

    let sheet = db
        .create_sheet(workbook_id, &sheet_name)
        .map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows
        .iter()
        .map(|r| r.iter().map(|v| v.as_str()).collect())
        .collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data)
        .map_err(|e| e.to_string())?;

    let wb_data = db
        .get_workbook_data(workbook_id)
        .map_err(|e| e.to_string())?;
    wb_data
        .sheets
        .into_iter()
        .find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub fn import_excel_file(
    db: State<'_, Database>,
    workbook_id: i64,
    file_path: String,
) -> Result<SheetData, String> {
    let path = security::validate_read_path(&file_path)?;
    let (sheet_name, headers, rows) = crate::import::import_excel(&path)?;

    let sheet = db
        .create_sheet(workbook_id, &sheet_name)
        .map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows
        .iter()
        .map(|r| r.iter().map(|v| v.as_str()).collect())
        .collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data)
        .map_err(|e| e.to_string())?;

    let wb_data = db
        .get_workbook_data(workbook_id)
        .map_err(|e| e.to_string())?;
    wb_data
        .sheets
        .into_iter()
        .find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub async fn import_google_sheet(
    db: State<'_, Database>,
    workbook_id: i64,
    spreadsheet_id: String,
    api_key: String,
    range: String,
) -> Result<SheetData, String> {
    let (headers, rows) =
        cloud_import::import_google_sheet(&spreadsheet_id, &api_key, &range).await?;
    let sheet_name = format!("Google-{}", &spreadsheet_id[..spreadsheet_id.len().min(8)]);
    let sheet = db
        .create_sheet(workbook_id, &sheet_name)
        .map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows
        .iter()
        .map(|r| r.iter().map(|v| v.as_str()).collect())
        .collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data)
        .map_err(|e| e.to_string())?;
    let wb_data = db
        .get_workbook_data(workbook_id)
        .map_err(|e| e.to_string())?;
    wb_data
        .sheets
        .into_iter()
        .find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub async fn import_notion_database(
    db: State<'_, Database>,
    workbook_id: i64,
    database_id: String,
    api_key: String,
) -> Result<SheetData, String> {
    let (headers, rows) = cloud_import::import_notion_database(&database_id, &api_key).await?;
    let sheet_name = format!("Notion-{}", &database_id[..database_id.len().min(8)]);
    let sheet = db
        .create_sheet(workbook_id, &sheet_name)
        .map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows
        .iter()
        .map(|r| r.iter().map(|v| v.as_str()).collect())
        .collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data)
        .map_err(|e| e.to_string())?;
    let wb_data = db
        .get_workbook_data(workbook_id)
        .map_err(|e| e.to_string())?;
    wb_data
        .sheets
        .into_iter()
        .find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub fn preview_import(path: String) -> Result<crate::models::ImportResult, String> {
    let p = security::validate_read_path(&path)?;
    match p.extension().and_then(|e| e.to_str()) {
        Some("csv") => crate::import::import_csv(&p),
        Some("xlsx") | Some("xls") => {
            let (sheet_name, columns, rows) = crate::import::import_excel(&p)?;
            let rows_imported = rows.len();
            Ok(ImportResult {
                rows_imported,
                columns,
                sheet_name,
            })
        }
        _ => Err("Unsupported file format. Use CSV or Excel files.".to_string()),
    }
}
