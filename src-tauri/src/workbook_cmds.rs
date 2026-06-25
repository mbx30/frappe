//! Workbook management commands (Phase 1.2).
//!
//! CRUD for workbooks, sheets, columns, and cell values.

use tauri::State;

use crate::db::Database;
use crate::models::{Sheet, SheetColumn, Workbook, WorkbookData};

#[tauri::command]
pub fn create_workbook(db: State<'_, Database>, name: String) -> Result<Workbook, String> {
    db.create_workbook(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_workbooks(db: State<'_, Database>) -> Result<Vec<Workbook>, String> {
    db.list_workbooks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_workbook(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_workbook(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_workbook(db: State<'_, Database>, id: i64) -> Result<WorkbookData, String> {
    db.get_workbook_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_sheet(
    db: State<'_, Database>,
    workbook_id: i64,
    name: String,
) -> Result<Sheet, String> {
    db.create_sheet(workbook_id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_column(
    db: State<'_, Database>,
    sheet_id: i64,
    name: String,
    col_type: String,
) -> Result<SheetColumn, String> {
    db.add_column(sheet_id, &name, &col_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_cell_value(
    db: State<'_, Database>,
    sheet_id: i64,
    row_index: i64,
    column_id: i64,
    value: String,
) -> Result<(), String> {
    db.update_cell(sheet_id, row_index, column_id, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_row(db: State<'_, Database>, sheet_id: i64) -> Result<i64, String> {
    db.add_row(sheet_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_workbook_name(db: State<'_, Database>, id: i64, name: String) -> Result<(), String> {
    db.update_workbook_name(id, &name)
        .map_err(|e| e.to_string())
}
