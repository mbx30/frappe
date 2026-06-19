use std::path::PathBuf;

use tauri::State;

use crate::db::{Database, VerificationResult};
use crate::models::{*, BusinessInfo};
use crate::cloud_import;

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
pub fn create_sheet(db: State<'_, Database>, workbook_id: i64, name: String) -> Result<Sheet, String> {
    db.create_sheet(workbook_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_column(db: State<'_, Database>, sheet_id: i64, name: String, col_type: String) -> Result<SheetColumn, String> {
    db.add_column(sheet_id, &name, &col_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_cell_value(db: State<'_, Database>, sheet_id: i64, row_index: i64, column_id: i64, value: String) -> Result<(), String> {
    db.update_cell(sheet_id, row_index, column_id, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_row(db: State<'_, Database>, sheet_id: i64) -> Result<i64, String> {
    db.add_row(sheet_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_workbook_name(db: State<'_, Database>, id: i64, name: String) -> Result<(), String> {
    db.update_workbook_name(id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_csv_file(db: State<'_, Database>, workbook_id: i64, file_path: String) -> Result<SheetData, String> {
    let path = PathBuf::from(&file_path);
    let (sheet_name, headers, rows) = crate::import::import_csv_data(&path)?;

    let sheet = db.create_sheet(workbook_id, &sheet_name).map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows.iter().map(|r| r.iter().map(|v| v.as_str()).collect()).collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data).map_err(|e| e.to_string())?;

    let wb_data = db.get_workbook_data(workbook_id).map_err(|e| e.to_string())?;
    wb_data.sheets.into_iter().find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub fn import_excel_file(db: State<'_, Database>, workbook_id: i64, file_path: String) -> Result<SheetData, String> {
    let path = PathBuf::from(&file_path);
    let (sheet_name, headers, rows) = crate::import::import_excel(&path)?;

    let sheet = db.create_sheet(workbook_id, &sheet_name).map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows.iter().map(|r| r.iter().map(|v| v.as_str()).collect()).collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data).map_err(|e| e.to_string())?;

    let wb_data = db.get_workbook_data(workbook_id).map_err(|e| e.to_string())?;
    wb_data.sheets.into_iter().find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub async fn import_google_sheet(db: State<'_, Database>, workbook_id: i64, spreadsheet_id: String, api_key: String, range: String) -> Result<SheetData, String> {
    let (headers, rows) = cloud_import::import_google_sheet(&spreadsheet_id, &api_key, &range).await?;
    let sheet_name = format!("Google-{}", &spreadsheet_id[..spreadsheet_id.len().min(8)]);
    let sheet = db.create_sheet(workbook_id, &sheet_name).map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows.iter().map(|r| r.iter().map(|v| v.as_str()).collect()).collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data).map_err(|e| e.to_string())?;
    let wb_data = db.get_workbook_data(workbook_id).map_err(|e| e.to_string())?;
    wb_data.sheets.into_iter().find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub async fn import_notion_database(db: State<'_, Database>, workbook_id: i64, database_id: String, api_key: String) -> Result<SheetData, String> {
    let (headers, rows) = cloud_import::import_notion_database(&database_id, &api_key).await?;
    let sheet_name = format!("Notion-{}", &database_id[..database_id.len().min(8)]);
    let sheet = db.create_sheet(workbook_id, &sheet_name).map_err(|e| e.to_string())?;
    let col_types: Vec<(&str, &str)> = headers.iter().map(|h| (h.as_str(), "text")).collect();
    let rows_data: Vec<Vec<&str>> = rows.iter().map(|r| r.iter().map(|v| v.as_str()).collect()).collect();
    db.replace_sheet_data(sheet.id, &col_types, &rows_data).map_err(|e| e.to_string())?;
    let wb_data = db.get_workbook_data(workbook_id).map_err(|e| e.to_string())?;
    wb_data.sheets.into_iter().find(|s| s.sheet.id == sheet.id)
        .ok_or_else(|| "Sheet not found after import".to_string())
}

#[tauri::command]
pub fn preview_import(path: String) -> Result<crate::models::ImportResult, String> {
    let p = PathBuf::from(&path);
    match p.extension().and_then(|e| e.to_str()) {
        Some("csv") => crate::import::import_csv(&p),
        Some("xlsx") | Some("xls") => {
            let (sheet_name, columns, _) = crate::import::import_excel(&p)?;
            let rows_imported = 0; // We'll count in the frontend
            Ok(ImportResult { rows_imported, columns, sheet_name })
        }
        _ => Err("Unsupported file format. Use CSV or Excel files.".to_string()),
    }
}

#[tauri::command]
pub fn verify_database(db: State<'_, Database>) -> VerificationResult {
    db.verify_integrity()
}

#[tauri::command]
pub fn get_business_info(db: State<'_, Database>) -> Result<Option<BusinessInfo>, String> {
    db.get_business_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_business_info(db: State<'_, Database>, business_name: String, industry: String, company_size: String) -> Result<(), String> {
    db.save_business_info(&business_name, &industry, &company_size).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_invoice(db: State<'_, Database>, invoice_number: String, due_date: String, payment_terms: String) -> Result<Invoice, String> {
    db.create_invoice(&invoice_number, &due_date, &payment_terms).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_invoices(db: State<'_, Database>) -> Result<Vec<Invoice>, String> {
    db.list_invoices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_invoice(db: State<'_, Database>, id: i64) -> Result<InvoiceData, String> {
    db.get_invoice_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_invoice_line_item(db: State<'_, Database>, invoice_id: i64, description: String, quantity: f64, unit_price: f64) -> Result<InvoiceLineItem, String> {
    db.add_line_item(invoice_id, &description, quantity, unit_price).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_invoice(db: State<'_, Database>, id: i64, status: String, subtotal: f64, tax_rate: f64, tax_amount: f64, total: f64, internal_notes: String, customer_notes: String) -> Result<(), String> {
    db.update_invoice(id, &status, subtotal, tax_rate, tax_amount, total, &internal_notes, &customer_notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_order(db: State<'_, Database>, order_number: String, due_date: String, description: String) -> Result<Order, String> {
    db.create_order(&order_number, &due_date, &description).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_orders(db: State<'_, Database>) -> Result<Vec<Order>, String> {
    db.list_orders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_order(db: State<'_, Database>, id: i64) -> Result<OrderData, String> {
    db.get_order_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order_status(db: State<'_, Database>, order_id: i64, new_status: String, notes: String) -> Result<(), String> {
    db.update_order_status(order_id, &new_status, &notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order(db: State<'_, Database>, id: i64, priority: String, description: String, artwork_notes: String, artwork_approved: bool, deposit_requested: bool, deposit_amount: f64, total_value: f64) -> Result<(), String> {
    db.update_order(id, &priority, &description, &artwork_notes, artwork_approved, deposit_requested, deposit_amount, total_value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_estimate(db: State<'_, Database>, estimate_number: String, valid_until: String) -> Result<Estimate, String> {
    db.create_estimate(&estimate_number, &valid_until).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_estimates(db: State<'_, Database>) -> Result<Vec<Estimate>, String> {
    db.list_estimates().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_estimate(db: State<'_, Database>, id: i64) -> Result<EstimateData, String> {
    db.get_estimate_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_estimate_line_item(db: State<'_, Database>, estimate_id: i64, description: String, category: String, quantity: f64, unit_price: f64) -> Result<EstimateLineItem, String> {
    db.add_estimate_line_item(estimate_id, &description, &category, quantity, unit_price).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_estimate(db: State<'_, Database>, id: i64, status: String, subtotal: f64, tax_rate: f64, tax_amount: f64, total: f64, notes: String, artwork_requirements: String) -> Result<(), String> {
    db.update_estimate(id, &status, subtotal, tax_rate, tax_amount, total, &notes, &artwork_requirements).map_err(|e| e.to_string())
}
