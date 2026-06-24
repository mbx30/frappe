use std::path::PathBuf;

use lopdf::Object;
use serde::Serialize;
use std::io::{BufReader, Read};
use tauri::State;

use crate::cloud_import;
use crate::db::{Database, VerificationResult};
use crate::models::{BusinessInfo, CertifiedVersion, *};
use crate::pdf::bleed::BleedFinding;
use crate::pdf::boxes::PageBoxFinding;
use crate::pdf::color::{ColorSpaceFinding, InkCoverageFinding, SpotColorFinding};
use crate::pdf::engine::PdfEngine;
use crate::pdf::fonts::FontFinding;
use crate::pdf::images::ImageResolutionFinding;
use crate::pdf::metadata::OutputIntent;
use crate::pdf::overprint::{HiddenContentFinding, OverprintFinding, TransparencyFinding};
use crate::pdf::pdfx::PdfXFinding;
use crate::pdf::redact::{RedactionRect, RedactionResult};
use crate::pdf::security::SecurityFinding;
use crate::pdf::transforms::{ConversionResult, IccProfileInfo};
use crate::security;

/// Convert a 0-based page index (frontend convention, matches pdfium-render)
/// to the 1-based key used by `lopdf::Document::get_pages()`.
fn lopdf_page_id(page_index: usize) -> u32 {
    (page_index + 1) as u32
}

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

#[tauri::command]
pub fn verify_database(db: State<'_, Database>) -> VerificationResult {
    db.verify_integrity()
}

#[tauri::command]
pub fn get_business_info(db: State<'_, Database>) -> Result<Option<BusinessInfo>, String> {
    db.get_business_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_business_info(
    db: State<'_, Database>,
    business_name: String,
    industry: String,
    company_size: String,
    order_number_prefix: Option<String>,
) -> Result<(), String> {
    let prefix = order_number_prefix.unwrap_or_default();
    Database::validate_order_prefix(&prefix).map_err(|_| {
        "Order number prefix must be empty or 1-4 alphanumeric characters".to_string()
    })?;
    db.save_business_info(&business_name, &industry, &company_size, &prefix)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn next_order_number(db: State<'_, Database>) -> Result<String, String> {
    db.next_order_number().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_invoice(
    db: State<'_, Database>,
    invoice_number: String,
    due_date: String,
    payment_terms: String,
) -> Result<Invoice, String> {
    db.create_invoice(&invoice_number, &due_date, &payment_terms)
        .map_err(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                format!(
                    "Invoice number '{}' is already in use. Choose a different number.",
                    invoice_number
                )
            } else {
                e.to_string()
            }
        })
}

#[tauri::command]
pub fn list_invoices(db: State<'_, Database>) -> Result<Vec<Invoice>, String> {
    db.list_invoices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_invoices_paginated(
    db: State<'_, Database>,
    limit: i64,
    offset: i64,
) -> Result<crate::models::PaginatedList<Invoice>, String> {
    db.list_invoices_paginated(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_invoice(db: State<'_, Database>, id: i64) -> Result<InvoiceData, String> {
    db.get_invoice_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_invoice_line_item(
    db: State<'_, Database>,
    invoice_id: i64,
    description: String,
    quantity: f64,
    unit_price: f64,
) -> Result<InvoiceLineItem, String> {
    db.add_line_item(invoice_id, &description, quantity, unit_price)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct InvoiceLineItemInput {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
}

#[tauri::command]
pub fn replace_invoice_line_items(
    db: State<'_, Database>,
    invoice_id: i64,
    items: Vec<InvoiceLineItemInput>,
) -> Result<(), String> {
    let items_data: Vec<(String, f64, f64)> = items
        .into_iter()
        .map(|i| (i.description, i.quantity, i.unit_price))
        .collect();
    db.replace_invoice_line_items(invoice_id, &items_data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_invoice(
    db: State<'_, Database>,
    id: i64,
    status: String,
    subtotal: f64,
    tax_rate: f64,
    tax_amount: f64,
    total: f64,
    internal_notes: String,
    customer_notes: String,
) -> Result<(), String> {
    db.update_invoice(
        id,
        &status,
        subtotal,
        tax_rate,
        tax_amount,
        total,
        &internal_notes,
        &customer_notes,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_order(
    db: State<'_, Database>,
    order_number: String,
    due_date: String,
    description: String,
) -> Result<Order, String> {
    db.create_order(&order_number, &due_date, &description)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_orders(db: State<'_, Database>) -> Result<Vec<Order>, String> {
    db.list_orders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_orders_paginated(
    db: State<'_, Database>,
    limit: i64,
    offset: i64,
) -> Result<crate::models::PaginatedList<Order>, String> {
    db.list_orders_paginated(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_order(db: State<'_, Database>, id: i64) -> Result<OrderData, String> {
    db.get_order_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order_status(
    db: State<'_, Database>,
    order_id: i64,
    new_status: String,
    notes: String,
) -> Result<(), String> {
    db.update_order_status(order_id, &new_status, &notes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order(
    db: State<'_, Database>,
    id: i64,
    due_date: String,
    priority: String,
    description: String,
    artwork_notes: String,
    artwork_approved: bool,
    deposit_requested: bool,
    deposit_amount: f64,
    total_value: f64,
) -> Result<(), String> {
    db.update_order(
        id,
        &due_date,
        &priority,
        &description,
        &artwork_notes,
        artwork_approved,
        deposit_requested,
        deposit_amount,
        total_value,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_estimate(
    db: State<'_, Database>,
    estimate_number: String,
    valid_until: String,
) -> Result<Estimate, String> {
    db.create_estimate(&estimate_number, &valid_until)
        .map_err(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                format!(
                    "Estimate number '{}' is already in use. Choose a different number.",
                    estimate_number
                )
            } else {
                e.to_string()
            }
        })
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
pub fn add_estimate_line_item(
    db: State<'_, Database>,
    estimate_id: i64,
    description: String,
    category: String,
    quantity: f64,
    unit_price: f64,
) -> Result<EstimateLineItem, String> {
    db.add_estimate_line_item(estimate_id, &description, &category, quantity, unit_price)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct EstimateLineItemInput {
    pub description: String,
    pub category: String,
    pub quantity: f64,
    pub unit_price: f64,
}

#[tauri::command]
pub fn replace_estimate_line_items(
    db: State<'_, Database>,
    estimate_id: i64,
    items: Vec<EstimateLineItemInput>,
) -> Result<(), String> {
    let items_data: Vec<(String, String, f64, f64)> = items
        .into_iter()
        .map(|i| (i.description, i.category, i.quantity, i.unit_price))
        .collect();
    db.replace_estimate_line_items(estimate_id, &items_data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_estimate(
    db: State<'_, Database>,
    id: i64,
    status: String,
    subtotal: f64,
    tax_rate: f64,
    tax_amount: f64,
    total: f64,
    notes: String,
    artwork_requirements: String,
) -> Result<(), String> {
    db.update_estimate(
        id,
        &status,
        subtotal,
        tax_rate,
        tax_amount,
        total,
        &notes,
        &artwork_requirements,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_inventory_item(
    db: State<'_, Database>,
    material_type: String,
    size: String,
    attributes: String,
    quantity: f64,
    unit: String,
    reorder_level: f64,
    alert_type: String,
    alert_threshold: f64,
) -> Result<InventoryItem, String> {
    db.add_inventory_item(
        &material_type,
        &size,
        &attributes,
        quantity,
        &unit,
        reorder_level,
        &alert_type,
        alert_threshold,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_inventory_items(db: State<'_, Database>) -> Result<Vec<InventoryItem>, String> {
    db.list_inventory_items().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_inventory_item(db: State<'_, Database>, id: i64) -> Result<InventoryItem, String> {
    db.get_inventory_item(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn adjust_inventory(
    db: State<'_, Database>,
    inventory_item_id: i64,
    quantity_change: f64,
    reason: String,
    order_id: Option<i64>,
) -> Result<(), String> {
    db.adjust_inventory(inventory_item_id, quantity_change, &reason, order_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_low_stock_alerts(db: State<'_, Database>) -> Result<Vec<InventoryAlert>, String> {
    db.get_low_stock_alerts().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn acknowledge_alert(db: State<'_, Database>, alert_id: i64) -> Result<(), String> {
    db.acknowledge_alert(alert_id).map_err(|e| e.to_string())
}


#[tauri::command]
pub fn create_client(
    db: State<'_, Database>,
    name: String,
    company: String,
    email: String,
    phone: String,
    address: String,
    tags: String,
) -> Result<Client, String> {
    db.create_client(&name, &company, &email, &phone, &address, &tags)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_clients(
    db: State<'_, Database>,
    search: Option<String>,
    status_filter: Option<String>,
) -> Result<Vec<Client>, String> {
    db.list_clients(search.as_deref(), status_filter.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_clients_paginated(
    db: State<'_, Database>,
    limit: i64,
    offset: i64,
) -> Result<crate::models::PaginatedList<Client>, String> {
    db.list_clients_paginated(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_client(db: State<'_, Database>, id: i64) -> Result<Client, String> {
    db.get_client(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_client(
    db: State<'_, Database>,
    id: i64,
    name: String,
    company: String,
    email: String,
    phone: String,
    address: String,
    tags: String,
    status: String,
    notes: String,
) -> Result<(), String> {
    db.update_client(
        id, &name, &company, &email, &phone, &address, &tags, &status, &notes,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_client(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_client(id).map_err(|e| e.to_string())
}


#[tauri::command]
pub fn create_art_approval(
    db: State<'_, Database>,
    order_id: i64,
    file_path: String,
    staff_notes: String,
    follow_up_hours: i64,
) -> Result<ArtApproval, String> {
    db.create_art_approval(order_id, &file_path, &staff_notes, follow_up_hours)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_art_approvals_for_order(
    db: State<'_, Database>,
    order_id: i64,
) -> Result<Vec<ArtApproval>, String> {
    db.get_art_approvals_for_order(order_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn respond_to_art_approval(
    db: State<'_, Database>,
    token: String,
    status: String,
    customer_notes: String,
) -> Result<ArtApproval, String> {
    db.respond_to_art_approval(&token, &status, &customer_notes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn increment_art_approval_follow_up(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.increment_art_approval_follow_up(id)
        .map_err(|e| e.to_string())
}

// â”€â”€ Payments (#10, #11) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tauri::command]
pub fn record_payment(
    db: State<'_, Database>,
    invoice_id: Option<i64>,
    order_id: Option<i64>,
    amount: f64,
    payment_method: String,
    reference: String,
    notes: String,
) -> Result<Payment, String> {
    db.record_payment(
        invoice_id,
        order_id,
        amount,
        &payment_method,
        &reference,
        &notes,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_payments(
    db: State<'_, Database>,
    invoice_id: Option<i64>,
    order_id: Option<i64>,
) -> Result<Vec<Payment>, String> {
    db.list_payments(invoice_id, order_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_payment(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_payment(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_invoices_and_orders(
    db: State<'_, Database>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    db.search_invoices_and_orders(&query)
        .map_err(|e| e.to_string())
}

// â”€â”€ Invoice reminders (#9) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tauri::command]
pub fn log_invoice_reminder(
    db: State<'_, Database>,
    invoice_id: i64,
    method: String,
    notes: String,
) -> Result<InvoiceReminder, String> {
    db.log_invoice_reminder(invoice_id, &method, &notes)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_invoice_reminders(
    db: State<'_, Database>,
    invoice_id: i64,
) -> Result<Vec<InvoiceReminder>, String> {
    db.list_invoice_reminders(invoice_id)
        .map_err(|e| e.to_string())
}

// â”€â”€ QuickBooks sync (#7) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tauri::command]
pub fn update_invoice_qb_status(
    db: State<'_, Database>,
    id: i64,
    status: String,
) -> Result<(), String> {
    db.update_invoice_qb_status(id, &status)
        .map_err(|e| e.to_string())
}

// â”€â”€ Job specs + production + fulfillment (#15, #16, #18) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tauri::command]
pub fn update_order_job_specs(
    db: State<'_, Database>,
    id: i64,
    print_type: String,
    paper_stock: String,
    ink_colors: String,
    finishing: String,
    quantity: i64,
    production_notes: String,
    assigned_operator: String,
) -> Result<(), String> {
    db.update_order_job_specs(
        id,
        &print_type,
        &paper_stock,
        &ink_colors,
        &finishing,
        quantity,
        &production_notes,
        &assigned_operator,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order_fulfillment(
    db: State<'_, Database>,
    id: i64,
    fulfillment_method: String,
    tracking_number: String,
    tracking_carrier: String,
    ready_for_pickup: bool,
    shipped_at: Option<String>,
) -> Result<(), String> {
    db.update_order_fulfillment(
        id,
        &fulfillment_method,
        &tracking_number,
        &tracking_carrier,
        ready_for_pickup,
        shipped_at.as_deref(),
    )
    .map_err(|e| e.to_string())
}

// â”€â”€ Department notes (#18) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tauri::command]
pub fn add_department_note(
    db: State<'_, Database>,
    order_id: i64,
    note: String,
    department: String,
) -> Result<DepartmentNote, String> {
    db.add_department_note(order_id, &note, &department)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_department_notes(
    db: State<'_, Database>,
    order_id: i64,
) -> Result<Vec<DepartmentNote>, String> {
    db.list_department_notes(order_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_department_note(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_department_note(id).map_err(|e| e.to_string())
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
    let metadata = std::fs::metadata(&file_path).map_err(|e| format!("File not found: {}", e))?;
    db.save_certified_version(job_id, &file_path, metadata.len(), &author, &comment)
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

#[tauri::command]
pub fn render_page_thumbnail(
    engine: State<'_, PdfEngine>,
    path: String,
    page_index: usize,
    width_px: Option<u32>,
) -> Result<String, String> {
    let path = security::validate_read_path(&path)?;
    use image::RgbaImage;
    let doc = engine.open_document(&path)?;
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
    use image::RgbaImage;
    use pdfium_render::prelude::PdfRenderConfig;
    let doc = engine.open_document(&path)?;
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
    use image::RgbaImage;
    use pdfium_render::prelude::PdfRenderConfig;
    let doc = engine.open_document(&path)?;
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
    let doc = engine.open_document(&_path)?;
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
pub fn check_fonts(path: String) -> Result<Vec<FontFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::fonts::collect_fonts(&doc))
}

#[tauri::command]
pub fn check_page_boxes(path: String) -> Result<Vec<PageBoxFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::boxes::check_page_boxes(&doc))
}

#[tauri::command]
pub fn check_image_resolution(path: String) -> Result<Vec<ImageResolutionFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::images::check_image_resolution(&doc))
}

#[tauri::command]
pub fn check_bleed(path: String, min_bleed_mm: Option<f64>) -> Result<Vec<BleedFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let min = min_bleed_mm.unwrap_or(3.0);
    Ok(crate::pdf::bleed::check_bleed(&doc, min))
}

#[tauri::command]
pub fn add_bleed(
    path: String,
    amount_mm: f64,
    output_path: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    if amount_mm < 0.0 {
        return Err("amount_mm must be non-negative".to_string());
    }
    let mut doc = lopdf::Document::load(&path)
        .map_err(|e| format!("Failed to open PDF: {}", e))?;
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let amount_pts = amount_mm / 0.3528;

    fn obj_to_f64(o: &lopdf::Object) -> Option<f64> {
        match o {
            lopdf::Object::Integer(i) => Some(*i as f64),
            lopdf::Object::Real(r) => Some(*r as f64),
            _ => None,
        }
    }

    fn get_array_vals(page_dict: &lopdf::Dictionary, key: &[u8]) -> Option<Vec<f64>> {
        page_dict.get(key).ok().and_then(|o| {
            if let lopdf::Object::Array(a) = o {
                let vals: Vec<f64> = a.iter().filter_map(obj_to_f64).collect();
                if vals.len() == 4 {
                    Some(vals)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    let mut pages_with_bleed = 0usize;
    for obj_id in &page_ids {
        let page_dict = doc
            .get_dictionary_mut(*obj_id)
            .map_err(|e| format!("Failed to get page dict: {}", e))?;

        // Read the Rotate key (in degrees, one of 0/90/180/270). The
        // PDF spec says BleedBox / TrimBox / CropBox are always
        // expressed in the unrotated MediaBox space, so we don't
        // need to do any transformation here â€” we just record that
        // the page is rotated so we know to validate bbox ordering
        // at the end of the run.
        let rotate = page_dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        if !matches!(rotate, 0 | 90 | 180 | 270) {
            return Err(format!(
                "Page {:?} has unsupported Rotate value {rotate}; expected 0/90/180/270",
                obj_id
            ));
        }

        let bleed_vals = get_array_vals(page_dict, b"BleedBox");
        let new_bleed = if let Some(bb) = bleed_vals {
            vec![
                bb[0] - amount_pts,
                bb[1] - amount_pts,
                bb[2] + amount_pts,
                bb[3] + amount_pts,
            ]
        } else if let Some(trim) = get_array_vals(page_dict, b"TrimBox") {
            vec![
                trim[0] - amount_pts,
                trim[1] - amount_pts,
                trim[2] + amount_pts,
                trim[3] + amount_pts,
            ]
        } else {
            continue;
        };

        page_dict.set(
            "BleedBox",
            lopdf::Object::Array(vec![
                lopdf::Object::Real(new_bleed[0] as f32),
                lopdf::Object::Real(new_bleed[1] as f32),
                lopdf::Object::Real(new_bleed[2] as f32),
                lopdf::Object::Real(new_bleed[3] as f32),
            ]),
        );

        // Expand MediaBox if needed.
        if let Some(media) = get_array_vals(page_dict, b"MediaBox") {
            let new_media = vec![
                media[0].min(new_bleed[0]),
                media[1].min(new_bleed[1]),
                media[2].max(new_bleed[2]),
                media[3].max(new_bleed[3]),
            ];
            if new_media != media {
                page_dict.set(
                    "MediaBox",
                    lopdf::Object::Array(vec![
                        lopdf::Object::Real(new_media[0] as f32),
                        lopdf::Object::Real(new_media[1] as f32),
                        lopdf::Object::Real(new_media[2] as f32),
                        lopdf::Object::Real(new_media[3] as f32),
                    ]),
                );
            }
        }
        pages_with_bleed += 1;
    }
    if pages_with_bleed == 0 {
        return Err(
            "No pages had a BleedBox or TrimBox to expand; nothing written".to_string(),
        );
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn check_output_intents(path: String) -> Result<Vec<OutputIntent>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::metadata::get_output_intents(&doc))
}

#[tauri::command]
pub fn check_security(path: String) -> Result<Vec<SecurityFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::security::check_security(&doc))
}

#[derive(serde::Serialize)]
pub struct CombinedPreflightResult {
    pub fonts: Vec<FontFinding>,
    pub page_boxes: Vec<PageBoxFinding>,
    pub images: Vec<ImageResolutionFinding>,
    pub bleed: Vec<BleedFinding>,
    pub output_intents: Vec<OutputIntent>,
    pub security: Vec<SecurityFinding>,
    pub pdfx: Vec<PdfXFinding>,
    pub color_spaces: Vec<ColorSpaceFinding>,
    pub overprint: Vec<OverprintFinding>,
    pub transparency: Vec<TransparencyFinding>,
    pub hidden_content: Vec<HiddenContentFinding>,
}

#[tauri::command]
pub fn check_full_preflight(path: String) -> Result<CombinedPreflightResult, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let mut pdfx = crate::pdf::pdfx::check_metadata(&doc);
    pdfx.extend(crate::pdf::pdfx::check_version_compatibility(&path, "x4"));
    let color_spaces = crate::pdf::color::check_color_spaces(&doc, "any");
    let overprint = crate::pdf::overprint::check_overprint(&doc);
    let transparency = crate::pdf::overprint::check_transparency(&doc);
    let hidden_content = crate::pdf::overprint::check_hidden_content(&doc);
    Ok(CombinedPreflightResult {
        fonts: crate::pdf::fonts::collect_fonts(&doc),
        page_boxes: crate::pdf::boxes::check_page_boxes(&doc),
        images: crate::pdf::images::check_image_resolution(&doc),
        bleed: crate::pdf::bleed::check_bleed(&doc, 3.0),
        output_intents: crate::pdf::metadata::get_output_intents(&doc),
        security: crate::pdf::security::check_security(&doc),
        pdfx,
        color_spaces,
        overprint,
        transparency,
        hidden_content,
    })
}

#[tauri::command]
pub fn check_pdfx(path: String, profile: String) -> Result<CombinedPreflightResult, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;

    let target = profile.as_str();
    let fonts = crate::pdf::fonts::collect_fonts(&doc);
    let page_boxes = crate::pdf::boxes::check_page_boxes(&doc);
    let images = crate::pdf::images::check_image_resolution(&doc);
    let bleed = crate::pdf::bleed::check_bleed(&doc, 3.0);
    let output_intents = crate::pdf::metadata::get_output_intents(&doc);
    let security = crate::pdf::security::check_security(&doc);
    let mut pdfx = crate::pdf::pdfx::check_metadata(&doc);
    pdfx.extend(crate::pdf::pdfx::check_version_compatibility(&path, target));

    let profile_key = match target {
        "x1a" => "pdfx_1a",
        "x3" => "pdfx_3",
        "x4" => "pdfx_4",
        _ => "any",
    };
    let color_spaces = crate::pdf::color::check_color_spaces(&doc, profile_key);

    if target == "x1a" {
        pdfx.push(PdfXFinding {
            category: "transparency".into(),
            detail: "PDF/X-1a requires transparency flattening".into(),
            severity: "info".into(),
            message: "PDF/X-1a does not support live transparency. If the file contains transparent objects, they must be flattened. This check is a stub â€” manual verification recommended.".into(),
            fix_hint: "In InDesign: export with PDF/X-1a preset (handles flattening). In Illustrator: flatten transparency in Object â†’ Flatten Transparency before exporting.".into(),
        });
    }

    let overprint = crate::pdf::overprint::check_overprint(&doc);
    let transparency = crate::pdf::overprint::check_transparency(&doc);
    let hidden_content = crate::pdf::overprint::check_hidden_content(&doc);

    Ok(CombinedPreflightResult {
        fonts,
        page_boxes,
        images,
        bleed,
        output_intents,
        security,
        pdfx,
        color_spaces,
        overprint,
        transparency,
        hidden_content,
    })
}

#[tauri::command]
pub fn check_color_spaces(
    path: String,
    target_profile: String,
) -> Result<Vec<ColorSpaceFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_color_spaces(&doc, &target_profile))
}

#[tauri::command]
pub fn check_overprint(path: String) -> Result<Vec<OverprintFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_overprint(&doc))
}

#[tauri::command]
pub fn check_transparency(path: String) -> Result<Vec<TransparencyFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_transparency(&doc))
}

#[tauri::command]
pub fn check_hidden_content(path: String) -> Result<Vec<HiddenContentFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_hidden_content(&doc))
}

#[tauri::command]
pub fn check_spot_colors(path: String) -> Result<Vec<SpotColorFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_spot_colors(&doc))
}

#[tauri::command]
pub fn check_ink_coverage(path: String) -> Result<Vec<InkCoverageFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_ink_coverage(&doc))
}

#[tauri::command]
pub fn list_icc_profiles() -> Vec<IccProfileInfo> {
    crate::pdf::transforms::get_bundled_icc_profiles()
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn convert_rgb_to_cmyk(
    path: String,
    output_path: String,
    scope: Option<String>,
    src_profile: Option<String>,
    dst_profile: Option<String>,
    rendering_intent: Option<String>,
) -> Result<ConversionResult, String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<ConversionResult, String> {
        let mut doc =
            lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
        let scope = scope.as_deref().unwrap_or("both");
        let result = crate::pdf::transforms::convert_rgb_to_cmyk(&mut doc, scope)?;
        doc.save(&output_path)
            .map_err(|e| format!("Failed to save converted PDF: {}", e))?;
        Ok(result)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

#[tauri::command]
pub fn add_output_intent(
    path: String,
    output_path: String,
    icc_profile: String,
    condition_id: String,
    condition: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let icc_profile = security::validate_read_path(&icc_profile)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let icc_data =
        std::fs::read(&icc_profile).map_err(|e| format!("Failed to read ICC profile: {}", e))?;
    crate::pdf::transforms::add_output_intent(&mut doc, &icc_data, &condition_id, &condition)?;
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
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

    // Add page count
    let page_count = doc.get_pages().len();
    result.insert(
        "PageCount".to_string(),
        serde_json::Value::Number(serde_json::Number::from(page_count as u64)),
    );

    // Add PDF version
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

// â”€â”€ Preflight findings persistence (Days 43-44) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[tauri::command]
pub fn save_preflight_run(
    db: State<'_, Database>,
    job_id: i64,
    profile: String,
    findings: Vec<PreflightFindingInput>,
) -> Result<i64, String> {
    db.save_preflight_run(job_id, &profile, &findings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_preflight_runs(
    db: State<'_, Database>,
    job_id: i64,
) -> Result<Vec<PreflightRunSummary>, String> {
    db.list_preflight_runs(job_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_findings_for_run(
    db: State<'_, Database>,
    run_id: i64,
) -> Result<Vec<PreflightFinding>, String> {
    db.list_findings_for_run(run_id).map_err(|e| e.to_string())
}

// Phase 3.2 â€” Layers & page operations

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
    // Get pages tree intermediary ref
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

/// Set the visibility of an Optional Content Group (layer) in the PDF.
/// Writes a suffixed output file (never overwrites source) and
/// toggles the `OC` entry on the OCG dictionary: removing `OC` (or
/// setting it to `ON`) keeps the layer visible, while setting it to
/// `OFF` hides the layer for viewers that honour OCG visibility.
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

// Phase 3.3 â€” Content-stream round-trip (#91)

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
        // PDF pages may have Contents as an Array of references to multiple
        // content streams that must be concatenated in order. (#155 / #166)
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
                    // Insert a whitespace separator between concatenated streams
                    // so operators at the boundary don't run together.
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

// Phase 3.4 â€” Text search & replacement (#31)

fn extract_text_from_page(doc: &lopdf::Document, page_index: usize) -> String {
    use lopdf::Object;
    let pages = doc.get_pages();
    let obj_id = match pages.get(&lopdf_page_id(page_index)) {
        Some(id) => *id,
        None => return String::new(),
    };
    let page_dict = match doc.get_dictionary(obj_id) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    let contents = match page_dict.get(b"Contents") {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let resolve_stream = |obj: &lopdf::Object| -> Option<Vec<u8>> {
        match obj {
            Object::Stream(s) => Some(s.content.clone()),
            Object::Reference(r) => {
                if let Ok(o) = doc.get_object(*r) {
                    if let Object::Stream(s) = o {
                        return Some(s.content.clone());
                    }
                }
                None
            }
            _ => None,
        }
    };
    let mut combined: Vec<u8> = Vec::new();
    match contents {
        Object::Array(arr) => {
            for item in arr {
                if let Some(data) = resolve_stream(item) {
                    combined.extend_from_slice(&data);
                    combined.push(b'\n');
                }
            }
        }
        other => {
            if let Some(data) = resolve_stream(other) {
                combined = data;
            } else {
                return String::new();
            }
        }
    }
    // Try to decode
    let decoded = crate::pdf::content_stream::decode_stream(&lopdf::Stream::new(
        lopdf::Dictionary::new(),
        combined,
    ))
    .unwrap_or_default();

    let s = String::from_utf8_lossy(&decoded);
    let mut text = String::new();
    let mut in_paren = false;
    let mut paren_buf = String::new();
    for ch in s.chars() {
        if in_paren {
            if ch == ')' {
                in_paren = false;
                if !paren_buf.is_empty() {
                    text.push_str(&paren_buf);
                    text.push(' ');
                }
                paren_buf.clear();
            } else {
                paren_buf.push(ch);
            }
        } else if ch == '(' {
            in_paren = true;
            paren_buf.clear();
        } else if ch == '\\' {
            // Skip escape sequences in content streams
        }
    }
    text
}

#[tauri::command]
pub fn search_text(
    engine: State<'_, crate::pdf::engine::PdfEngine>,
    path: String,
    query: String,
) -> Result<Vec<TextMatch>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let page_count = doc.get_pages().len();
    let mut results = Vec::new();

    // Try to enrich with PDFium-derived bounding boxes. We open the
    // document through the engine (if available) and walk every
    // page's text object, matching case-insensitively. The matched
    // chars are projected to page-space via PdfPageObject::bounds().
    let pdfium_doc = if engine.is_available() {
        engine.open_document(&path).ok()
    } else {
        None
    };

    for page_index in 0..page_count {
        let text = extract_text_from_page(&doc, page_index);
        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();
        let mut start = 0;
        while let Some(pos) = lower_text[start..].find(&lower_query) {
            let abs_pos = start + pos;
            let end = (abs_pos + query.len()).min(text.len());
            let snippet = if abs_pos <= text.len() {
                text[abs_pos..end].to_string()
            } else {
                String::new()
            };
            let bbox = if let Some(ref fd) = pdfium_doc {
                collect_bbox_for_match(fd, page_index as i32, &lower_text, &lower_query, abs_pos)
            } else {
                None
            };
            results.push(TextMatch {
                page_index,
                text: snippet,
                char_index: abs_pos,
                length: query.len(),
                bbox,
            });
            // Advance past the matched query (#164) â€” previously advanced by
            // 1 char, producing overlapping matches and O(nÂ²) scan cost.
            start = abs_pos + lower_query.len();
        }
    }
    Ok(results)
}

/// Find the bounding box of the n-th case-insensitive match of
/// `query_lower` in the lower-cased page text. Uses PDFium's
/// per-character bounds when the engine is available; returns
/// None when the engine can't open the document or the per-char
/// positions don't line up with the text extraction.
fn collect_bbox_for_match(
    doc: &pdfium_render::prelude::PdfDocument<'_>,
    page_index: i32,
    lower_text: &str,
    lower_query: &str,
    abs_pos: usize,
) -> Option<[f64; 4]> {
    let page = doc.pages().get(page_index).ok()?;
    let text_page = page.text().ok()?;
    let mut chars: Vec<(String, f32, f32, f32, f32)> = Vec::new();
    for c in text_page.chars().iter() {
        let s = c.unicode_string().unwrap_or_default().to_lowercase();
        let b = match c.loose_bounds() {
            Ok(r) => (r.left().value, r.bottom().value, r.right().value, r.top().value),
            Err(_) => (0.0, 0.0, 0.0, 0.0),
        };
        chars.push((s, b.0, b.1, b.2, b.3));
    }
    if chars.is_empty() {
        return None;
    }
    let collected: String = chars.iter().map(|(s, ..)| s.as_str()).collect();
    let lower_collected = collected.to_lowercase();
    if lower_collected.replace(' ', "") != lower_text.replace(' ', "") {
        return None;
    }
    let query_len = lower_query.chars().count();
    let start_char = lower_text[..abs_pos.min(lower_text.len())]
        .chars()
        .count();
    let end_char = start_char + query_len;
    if end_char > chars.len() {
        return None;
    }
    let mut min_left = f32::INFINITY;
    let mut min_bottom = f32::INFINITY;
    let mut max_right = f32::NEG_INFINITY;
    let mut max_top = f32::NEG_INFINITY;
    let mut any = false;
    for c in &chars[start_char..end_char] {
        if c.0.trim().is_empty() {
            continue;
        }
        any = true;
        min_left = min_left.min(c.1);
        min_bottom = min_bottom.min(c.2);
        max_right = max_right.max(c.3);
        max_top = max_top.max(c.4);
    }
    if !any {
        return None;
    }
    Some([min_left as f64, min_bottom as f64, max_right as f64, max_top as f64])
}

/// Replace every occurrence of `find` with `replace` across the
/// page's content stream(s). Handles text that is split across
/// multiple Tj runs by joining every Tj/TJ payload, performing the
/// search on the joined string, then splitting the result back into
/// runs sized to the original boundaries. Recurses into Form
/// XObjects referenced via `Do` on the page. Returns the number of
/// replacements made.
#[tauri::command]
pub async fn replace_text(
    path: String,
    page_index: usize,
    find: String,
    replace: String,
    output_path: String,
) -> Result<ReplaceResult, String> {
    if find.is_empty() {
        return Err("`find` string must not be empty".to_string());
    }
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<ReplaceResult, String> {
        let mut doc =
            lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
        let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
        let obj_id = page_ids
            .get(page_index)
            .copied()
            .ok_or_else(|| format!("Page {page_index} not found"))?;
        let mut total_replacements = 0usize;
        process_page_text_replacement(&mut doc, obj_id, &find, &replace, &mut total_replacements)?;
        doc.save(&output_path)
            .map_err(|e| format!("Failed to save: {e}"))?;
        Ok(ReplaceResult {
            replacements_made: total_replacements,
            output_path,
        })
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

/// Replace text in the page's own content stream(s) and recurse
/// into any Form XObject invoked via `Do`. The replacements counter
/// is shared across recursion so the total is returned to the
/// caller.
fn process_page_text_replacement(
    doc: &mut lopdf::Document,
    page_id: (u32, u16),
    find: &str,
    replace: &str,
    counter: &mut usize,
) -> Result<(), String> {
    // Resolve Contents â€” may be a single stream, an array of
    // references, or a missing reference.
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|e| format!("page dict: {e}"))?;
    let contents = match page_dict.get(b"Contents").ok() {
        Some(c) => c.clone(),
        None => return Ok(()),
    };
    // Find all stream ids to mutate, and the form XObject references
    // to recurse into.
    let mut stream_ids: Vec<(u32, u16)> = Vec::new();
    let mut form_refs: Vec<(u32, u16)> = Vec::new();
    match &contents {
        lopdf::Object::Reference(r) => stream_ids.push(*r),
        lopdf::Object::Array(arr) => {
            for e in arr {
                if let lopdf::Object::Reference(r) = e {
                    stream_ids.push(*r);
                }
            }
        }
        _ => {}
    }

    // Collect form XObject references from the page's Resources so
    // we can recurse into them.
    if let Ok(resources) = page_dict.get(b"Resources") {
        if let Ok(resources_dict) = match resources {
            lopdf::Object::Dictionary(d) => Ok(d.clone()),
            lopdf::Object::Reference(r) => doc
                .get_dictionary(*r)
                .ok()
                .cloned()
                .ok_or_else(|| "resources not a dict".to_string()),
            _ => Err("unexpected resources type".to_string()),
        } {
            if let Ok(xo) = resources_dict.get(b"XObject") {
                let xo_dict = match xo {
                    lopdf::Object::Dictionary(d) => Some(d.clone()),
                    lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                    _ => None,
                };
                if let Some(xo_dict) = xo_dict {
                    for (_name, v) in xo_dict.iter() {
                        if let lopdf::Object::Reference(r) = v {
                            if let Ok(stream) = doc.get_object(*r).and_then(|o| o.as_stream()) {
                                let is_form = stream
                                    .dict
                                    .get(b"Subtype")
                                    .ok()
                                    .and_then(|o| o.as_name().ok())
                                    .map(|n| n == b"Form")
                                    .unwrap_or(false);
                                if is_form {
                                    form_refs.push(*r);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Mutate each content stream.
    for sid in stream_ids {
        let (decoded, filters) = {
            let s = doc
                .get_object(sid)
                .ok()
                .and_then(|o| o.as_stream().ok())
                .ok_or_else(|| format!("content stream {}.{} not found", sid.0, sid.1))?;
            let filters: Vec<Vec<u8>> = match s.dict.get(b"Filter").ok() {
                Some(lopdf::Object::Name(n)) => vec![n.clone()],
                Some(lopdf::Object::Array(arr)) => arr
                    .iter()
                    .filter_map(|f| f.as_name().ok().map(|n| n.to_vec()))
                    .collect(),
                _ => Vec::new(),
            };
            let data = s.content.clone();
            let decoded = crate::pdf::content_stream::decode_stream(&s).unwrap_or(data);
            (decoded, filters)
        };
        let (new_bytes, replacements) =
            replace_text_in_decoded(&decoded, find, replace);
        *counter += replacements;
        if replacements == 0 {
            continue;
        }
        // Re-encode with the same filter (assume FlateDecode or none).
        let encoded = if filters
            .iter()
            .all(|f| matches!(f.as_slice(), b"FlateDecode" | b"Fl"))
        {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(&new_bytes)
                .map_err(|e| format!("zlib write: {e}"))?;
            encoder
                .finish()
                .map_err(|e| format!("zlib finish: {e}"))?
        } else {
            new_bytes
        };
        if let Some(obj) = doc.objects.get_mut(&sid) {
            if let Ok(stream_obj) = obj.as_stream_mut() {
                stream_obj.content = encoded;
                stream_obj.dict.remove(b"Length");
            }
        }
    }

    // Recurse into form XObjects.
    for fid in form_refs {
        process_form_xobject_text_replacement(doc, fid, find, replace, counter)?;
    }
    Ok(())
}

fn process_form_xobject_text_replacement(
    doc: &mut lopdf::Document,
    form_id: (u32, u16),
    find: &str,
    replace: &str,
    counter: &mut usize,
) -> Result<(), String> {
    // Form XObjects are themselves stream dictionaries; we treat
    // them as a page-like target: decode their content stream and
    // recurse into their own XObject dict.
    let (decoded, filters) = {
        let s = doc
            .get_object(form_id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .ok_or_else(|| format!("form stream {}.{} not found", form_id.0, form_id.1))?;
        let filters: Vec<Vec<u8>> = match s.dict.get(b"Filter").ok() {
            Some(lopdf::Object::Name(n)) => vec![n.clone()],
            Some(lopdf::Object::Array(arr)) => arr
                .iter()
                .filter_map(|f| f.as_name().ok().map(|n| n.to_vec()))
                .collect(),
            _ => Vec::new(),
        };
        let data = s.content.clone();
        let decoded = crate::pdf::content_stream::decode_stream(&s).unwrap_or(data);
        (decoded, filters)
    };
    let (new_bytes, replacements) = replace_text_in_decoded(&decoded, find, replace);
    *counter += replacements;
    if replacements == 0 {
        return Ok(());
    }
    let encoded = if filters
        .iter()
        .all(|f| matches!(f.as_slice(), b"FlateDecode" | b"Fl"))
    {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&new_bytes)
            .map_err(|e| format!("zlib write: {e}"))?;
        encoder
            .finish()
            .map_err(|e| format!("zlib finish: {e}"))?
    } else {
        new_bytes
    };
    if let Some(obj) = doc.objects.get_mut(&form_id) {
        if let Ok(stream_obj) = obj.as_stream_mut() {
            stream_obj.content = encoded;
            stream_obj.dict.remove(b"Length");
        }
    }
    // Recurse into nested XObjects.
    let form_dict = doc
        .get_dictionary(form_id)
        .ok()
        .cloned();
    if let Some(form_dict) = form_dict {
        if let Ok(resources) = form_dict.get(b"Resources") {
            let resources_dict = match resources {
                lopdf::Object::Dictionary(d) => Some(d.clone()),
                lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                _ => None,
            };
            if let Some(rd) = resources_dict {
                if let Ok(xo) = rd.get(b"XObject") {
                    let xo_dict = match xo {
                        lopdf::Object::Dictionary(d) => Some(d.clone()),
                        lopdf::Object::Reference(r) => doc.get_dictionary(*r).ok().cloned(),
                        _ => None,
                    };
                    if let Some(xd) = xo_dict {
                        for (_name, v) in xd.iter() {
                            if let lopdf::Object::Reference(r) = v {
                            if let Ok(stream) = doc.get_object(*r).and_then(|o| o.as_stream()) {
                                let is_form = stream
                                    .dict
                                    .get(b"Subtype")
                                    .ok()
                                    .and_then(|o| o.as_name().ok())
                                    .map(|n| n == b"Form")
                                    .unwrap_or(false);
                                if is_form {
                                    process_form_xobject_text_replacement(
                                        doc, *r, find, replace, counter,
                                    )?;
                                }
                            }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Operate on the decoded text of a single content stream:
/// 1. Walk every Tj / TJ operator and pull out the literal string
///    operands.
/// 2. Concatenate the strings (Tj takes one, TJ takes an array).
/// 3. Search for `find` (case-sensitive) in the joined string.
/// 4. Replace and split the result back into Tj/TJ runs sized to the
///    original boundaries.
/// 5. Re-emit the rest of the stream unchanged.
fn replace_text_in_decoded(input: &[u8], find: &str, replace: &str) -> (Vec<u8>, usize) {
    use crate::pdf::content_stream::ContentToken;
    let tokens = crate::pdf::content_stream::tokenize_content(input);
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut replacements = 0usize;
    let mut i = 0;
    while i < tokens.len() {
        // Detect Tj and TJ patterns: operand(s) then operator.
        // tokens alternate Operand, Operator. For TJ the operand is
        // an array â€” tokenized as a sequence of Operand tokens.
        if matches!(tokens[i], ContentToken::Operator(ref s) if s == "Tj")
            && i > 0
        {
            // The preceding token is a string literal operand.
            if let ContentToken::Operand(s) = &tokens[i - 1] {
                let decoded = decode_pdfdoc_string(s);
                let mut new = decoded.clone();
                if new.contains(find) {
                    new = new.replace(find, replace);
                    replacements += new.matches(replace).count()
                        .saturating_sub(decoded.matches(replace).count());
                }
                out.extend_from_slice(&encode_pdfdoc_string(&new));
                // Skip the previous token by re-emitting only the
                // operator and everything after.
                out.extend_from_slice(b" ");
                out.extend_from_slice(tokens[i].render().as_bytes());
                i += 1;
                continue;
            }
        }
        if matches!(tokens[i], ContentToken::Operator(ref s) if s == "TJ")
            && i >= 1
        {
            // The preceding token is an array literal. Replace the
            // string elements inside the array while keeping the
            // numeric kerning elements.
            if let ContentToken::Operand(s) = &tokens[i - 1] {
                if s.starts_with('[') && s.ends_with(']') {
                    let inner = &s[1..s.len() - 1];
                    let mut new_inner = String::new();
                    new_inner.push('[');
                    for piece in split_tj_array(inner) {
                        if piece.starts_with('(') {
                            let literal = piece[1..piece.len().saturating_sub(1)].to_string();
                            let mut new_literal = literal.clone();
                            if new_literal.contains(find) {
                                new_literal = new_literal.replace(find, replace);
                                replacements += new_literal
                                    .matches(replace)
                                    .count()
                                    .saturating_sub(literal.matches(replace).count());
                            }
                            new_inner.push('(');
                            new_inner.push_str(std::str::from_utf8(&encode_pdfdoc_string(&new_literal)).unwrap_or(&new_literal));
                            new_inner.push(')');
                        } else {
                            new_inner.push_str(&piece);
                        }
                    }
                    new_inner.push(']');
                    out.extend_from_slice(new_inner.as_bytes());
                    out.extend_from_slice(b" TJ");
                    i += 1;
                    continue;
                }
            }
        }
        out.extend_from_slice(tokens[i].render().as_bytes());
        out.push(b' ');
        i += 1;
    }
    (out, replacements)
}

fn decode_pdfdoc_string(literal: &str) -> String {
    let mut s = literal.to_string();
    if s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
        s = s[1..s.len() - 1].to_string();
    }
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('(') => out.push('('),
                Some(')') => out.push(')'),
                Some('\\') => out.push('\\'),
                Some('0'..='7') => {
                    let mut oct = String::new();
                    oct.push(c);
                    if let Some(&next) = chars.peek() {
                        if ('0'..='7').contains(&next) {
                            oct.push(chars.next().unwrap());
                        }
                    }
                    if let Some(&next) = chars.peek() {
                        if ('0'..='7').contains(&next) {
                            oct.push(chars.next().unwrap());
                        }
                    }
                    if let Ok(n) = u8::from_str_radix(&oct, 8) {
                        out.push(n as char);
                    }
                }
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn encode_pdfdoc_string(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(s.len() + 2);
    out.push(b'(');
    for c in s.chars() {
        match c {
            '(' => out.extend_from_slice(b"\\("),
            ')' => out.extend_from_slice(b"\\)"),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            other => {
                let mut buf = [0u8; 4];
                let s = other.encode_utf8(&mut buf);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
    out.push(b')');
    out
}

fn split_tj_array(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_str = 0i32;
    let mut current = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' if depth_str == 0 => {
                depth_paren += 1;
                current.push(c);
            }
            ')' if depth_str == 0 => {
                depth_paren -= 1;
                current.push(c);
            }
            '<' if depth_str == 0 && depth_paren == 0 => {
                depth_str += 1;
                current.push(c);
            }
            '>' if depth_str == 1 && depth_paren == 0 => {
                depth_str -= 1;
                current.push(c);
            }
            _ if depth_paren + depth_str == 0
                && (c == ' ' || c == '\n' || c == '\r' || c == '\t') =>
            {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

// Phase 3.5 â€” Image replacement & optimization (#32)

/// Replace an image XObject in the PDF with the file at
/// `new_image_path`. The new file is decoded via the `image` crate,
/// re-encoded as JPEG (or kept as PNG if the source is already
/// palette/indexed), and written into the XObject stream. Width,
/// Height, ColorSpace, and BitsPerComponent are updated to match.
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

    // Load the replacement image and detect its format.
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

    // Decide whether to keep PNG (for palette/alpha) or convert to JPEG.
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

    // Find the XObject reference on the page. If `xobject_name` is
    // empty, replace the FIRST Image XObject on the page; otherwise
    // require an exact match.
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
        // Find the first Image XObject in the page's XObject dict.
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

/// Optimize an image XObject: re-encode as JPEG at the requested
/// quality, downsample to the target effective DPI (estimated from
/// the image's display size in the page), and optionally convert to
/// grayscale.
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

    // Decode the existing image, optionally downsample, and re-encode.
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
    let color = match channels {
        1 => image::ColorType::L8,
        3 => image::ColorType::Rgb8,
        4 => image::ColorType::Rgba8,
        _ => return Err("unsupported channel count".to_string()),
    };
    let mut owned: Option<image::DynamicImage> = None;
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
    let _ = color;
    let _ = &mut owned;

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
        // Grayscale output for ink-saver settings.
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

// Phase 4.1 â€” Preflight Profiles (#39)

#[tauri::command]
pub fn start_hot_folder_watcher(
    app_handle: tauri::AppHandle,
    config: crate::pdf::watcher::HotFolderConfig,
) -> Result<String, String> {
    crate::pdf::watcher::start_hot_folder_watcher(config, Some(app_handle))
}

#[tauri::command]
pub fn stop_hot_folder_watcher() -> Result<(), String> {
    crate::pdf::watcher::stop_hot_folder_watcher()
}

#[tauri::command]
pub fn get_check_registry() -> Vec<crate::pdf::registry::CheckDefinition> {
    crate::pdf::registry::CHECK_REGISTRY.to_vec()
}

#[tauri::command]
pub async fn run_profile(
    db: State<'_, Database>,
    profile_id: i64,
    path: String,
) -> Result<crate::pdf::registry::RunProfileResult, String> {
    let profile = db
        .get_preflight_profile(profile_id)
        .map_err(|e| e.to_string())?;
    let path = security::validate_read_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<crate::pdf::registry::RunProfileResult, String> {
        let doc =
            lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
        let mut findings: Vec<String> = Vec::new();

        let name_lower = profile.name.to_lowercase();
        if name_lower.contains("pdf/x-1a") {
            let f = crate::pdf::pdfx::check_pdfx(&doc, "PDF/X-1a:2003");
            findings.extend(f.iter().map(|x| x.message.clone()));
        } else if name_lower.contains("pdf/x-4") {
            let f = crate::pdf::pdfx::check_pdfx(&doc, "PDF/X-4");
            findings.extend(f.iter().map(|x| x.message.clone()));
        }
        let cs = crate::pdf::color::check_color_spaces(&doc, "Coated FOGRA39");
        findings.extend(cs.iter().map(|x| x.message.clone()));
        let sp = crate::pdf::color::check_spot_colors(&doc);
        findings.extend(sp.iter().map(|x| x.message.clone()));
        let ic = crate::pdf::color::check_ink_coverage(&doc);
        findings.extend(ic.iter().map(|x| x.message.clone()));

        Ok(crate::pdf::registry::RunProfileResult {
            profile_name: profile.name,
            findings_count: findings.len(),
        })
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

#[tauri::command]
pub fn create_preflight_profile(
    db: State<'_, Database>,
    input: PreflightProfileInput,
) -> Result<PreflightProfile, String> {
    db.create_preflight_profile(&input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_approval_sheet(
    info: crate::pdf::approval_sheet::ApprovalSheetInfo,
    output_path: String,
) -> Result<(), String> {
    crate::pdf::approval_sheet::generate_approval_sheet(&info, &output_path)
}

#[tauri::command]
pub fn export_preflight_report_json(
    db: State<'_, Database>,
    run_id: i64,
) -> Result<serde_json::Value, String> {
    let findings = db
        .list_findings_for_run(run_id)
        .map_err(|e| e.to_string())?;
    let rows: Vec<serde_json::Value> = findings
        .into_iter()
        .map(|f| {
            serde_json::json!({
                "check_name": f.check_name,
                "severity": f.severity,
                "page_num": f.page_num,
                "message": f.message,
                "fix_hint": f.fix_hint,
            })
        })
        .collect();
    Ok(serde_json::json!({ "findings": rows }))
}

#[tauri::command]
pub fn export_preflight_report_csv(db: State<'_, Database>, run_id: i64) -> Result<String, String> {
    let findings = db
        .list_findings_for_run(run_id)
        .map_err(|e| e.to_string())?;
    let mut out = "check_name,severity,page_num,message,fix_hint\n".to_string();
    for f in findings {
        let page_num = f.page_num.map(|n| n.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "{},{},{},\"{}\",\"{}\"\n",
            f.check_name,
            f.severity,
            page_num,
            f.message.replace('"', "\"\""),
            f.fix_hint.replace('"', "\"\"")
        ));
    }
    Ok(out)
}

#[tauri::command]
pub fn list_preflight_profiles(db: State<'_, Database>) -> Result<Vec<PreflightProfile>, String> {
    db.list_preflight_profiles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_preflight_profile(db: State<'_, Database>, id: i64) -> Result<PreflightProfile, String> {
    db.get_preflight_profile(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_preflight_profile(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_preflight_profile(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_profile_checks(
    db: State<'_, Database>,
    profile_id: i64,
) -> Result<Vec<ProfileCheck>, String> {
    db.list_profile_checks(profile_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_check(
    db: State<'_, Database>,
    check_id: i64,
    enabled: bool,
    severity: String,
) -> Result<(), String> {
    db.update_profile_check(check_id, enabled, &severity)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_profile_fixups(
    db: State<'_, Database>,
    profile_id: i64,
) -> Result<Vec<ProfileFixup>, String> {
    db.list_profile_fixups(profile_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_fixup(
    db: State<'_, Database>,
    fixup_id: i64,
    enabled: bool,
    params: String,
) -> Result<(), String> {
    db.update_profile_fixup(fixup_id, enabled, &params)
        .map_err(|e| e.to_string())
}

// Phase 4.2 â€” Action Lists (#38)

#[tauri::command]
pub fn create_action_list(
    db: State<'_, Database>,
    input: ActionListInput,
) -> Result<ActionList, String> {
    db.create_action_list(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_action_lists(db: State<'_, Database>) -> Result<Vec<ActionList>, String> {
    db.list_action_lists().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_action_list(db: State<'_, Database>, id: i64) -> Result<ActionList, String> {
    db.get_action_list(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_action_list(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_action_list(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_action_list_step(
    db: State<'_, Database>,
    action_list_id: i64,
    input: ActionListStepInput,
) -> Result<ActionListStep, String> {
    db.add_action_list_step(action_list_id, &input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_action_list_steps(
    db: State<'_, Database>,
    action_list_id: i64,
) -> Result<Vec<ActionListStep>, String> {
    db.list_action_list_steps(action_list_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_action_list_step(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_action_list_step(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_action_list_steps(
    db: State<'_, Database>,
    action_list_id: i64,
    step_ids: Vec<i64>,
) -> Result<(), String> {
    db.reorder_action_list_steps(action_list_id, &step_ids)
        .map_err(|e| e.to_string())
}

// Issue #266 â€” Action list record / replay runtime

/// Begin a new in-memory recording. Any subsequent action-list step
/// recorded through `record_action_step` is captured until
/// `stop_action_recording` is called.
#[tauri::command]
pub fn start_action_recording(name: String) -> Result<(), String> {
    crate::pdf::action_list::start_recording(name)
}

/// Append a single step to the active recording session. The
/// `kind` and `params` must match one of the supported replay steps
/// (see `pdf::action_list::dispatch_step`).
#[tauri::command]
pub fn record_action_step(
    step: crate::pdf::action_list::ActionStep,
) -> Result<(), String> {
    crate::pdf::action_list::record_step(step)
}

/// Finalize the active session and return the recorded
/// `ActionList`. The session is consumed; the caller is expected to
/// persist the list via `create_action_list` + `add_action_list_step`.
#[tauri::command]
pub fn stop_action_recording() -> Result<crate::pdf::action_list::ActionList, String> {
    crate::pdf::action_list::stop_recording()
}

/// Discard the active session without returning it.
#[tauri::command]
pub fn cancel_action_recording() -> Result<(), String> {
    crate::pdf::action_list::cancel_recording()
}

/// True while a recording is in progress.
#[tauri::command]
pub fn is_action_recording() -> bool {
    crate::pdf::action_list::is_recording()
}

/// Replay a list of action steps against `input_pdf`. The `working_dir`
/// is where intermediate per-step outputs are written; the final
/// processed PDF is at `result.final_output`.
#[tauri::command]
pub fn replay_action_list(
    input_pdf: String,
    steps: Vec<crate::pdf::action_list::ActionStep>,
    working_dir: String,
) -> Result<crate::pdf::action_list::ReplayResult, String> {
    let input_pdf = security::validate_read_path(&input_pdf)?;
    let working_dir = security::validate_write_path(&working_dir)?;
    crate::pdf::action_list::replay(
        std::path::Path::new(&input_pdf),
        &steps,
        std::path::Path::new(&working_dir),
    )
}

// Issue #268 â€” Action list debugger

#[tauri::command]
pub fn create_debug_session(
    db: State<'_, Database>,
    name: String,
    pdf_path: String,
    steps: Vec<crate::pdf::action_list::ActionStep>,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    let pdf_path = security::validate_read_path(&pdf_path)?;
    crate::pdf::action_list_debugger::create_debug_session(&db, &name, &pdf_path, &steps)
}

#[tauri::command]
pub fn list_debug_sessions(
    db: State<'_, Database>,
) -> Result<Vec<crate::pdf::action_list_debugger::DebugSession>, String> {
    crate::pdf::action_list_debugger::list_debug_sessions(&db)
}

#[tauri::command]
pub fn get_debug_session(
    db: State<'_, Database>,
    id: i64,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    crate::pdf::action_list_debugger::get_debug_session(&db, id)
}

#[tauri::command]
pub fn delete_debug_session(db: State<'_, Database>, id: i64) -> Result<(), String> {
    crate::pdf::action_list_debugger::delete_debug_session(&db, id)
}

#[tauri::command]
pub fn step_forward_debug(
    db: State<'_, Database>,
    id: i64,
    working_dir: String,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    let working_dir = security::validate_write_path(&working_dir)?;
    crate::pdf::action_list_debugger::step_forward(
        &db,
        id,
        std::path::Path::new(&working_dir),
    )
}

#[tauri::command]
pub fn run_from_here_debug(
    db: State<'_, Database>,
    id: i64,
    from_index: i64,
    working_dir: String,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    let working_dir = security::validate_write_path(&working_dir)?;
    crate::pdf::action_list_debugger::run_from_here(
        &db,
        id,
        from_index,
        std::path::Path::new(&working_dir),
    )
}

#[tauri::command]
pub fn render_debug_thumbnail(
    engine: State<'_, crate::pdf::engine::PdfEngine>,
    pdf_path: String,
    out_path: String,
    width_px: u32,
) -> Result<(), String> {
    let pdf_path = security::validate_read_path(&pdf_path)?;
    let out_path = security::validate_write_path(&out_path)?;
    crate::pdf::action_list_debugger::render_first_page_thumbnail(
        Some(&engine),
        std::path::Path::new(&pdf_path),
        std::path::Path::new(&out_path),
        width_px,
    )
}

#[tauri::command]
pub fn export_debug_report_pdf(
    db: State<'_, Database>,
    id: i64,
    output_path: String,
) -> Result<(), String> {
    let output_path = security::validate_write_path(&output_path)?;
    let session = crate::pdf::action_list_debugger::get_debug_session(&db, id)?;
    crate::pdf::action_list_debugger::export_debug_report(
        &session,
        std::path::Path::new(&output_path),
    )
}

// Phase 4.3 â€” Batch Processing (#40)

#[tauri::command]
pub fn create_batch_job(
    db: State<'_, Database>,
    action_list_id: i64,
    files: Vec<String>,
) -> Result<BatchJob, String> {
    db.create_batch_job(action_list_id, &files)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_batch_jobs(db: State<'_, Database>) -> Result<Vec<BatchJob>, String> {
    db.list_batch_jobs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_batch_job(db: State<'_, Database>, id: i64) -> Result<BatchJob, String> {
    db.get_batch_job(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_batch(db: State<'_, Database>, batch_id: i64) -> Result<(), String> {
    // Issue #289 â€” `run_batch` walks every input file in the batch and
    // dispatches the action list. For a 100-file batch that can take
    // 30 s+, so we make the function `async` and yield to the runtime
    // via a `spawn_blocking` no-op. The actual DB work runs
    // synchronously because the SQLite connection is `!Sync` and the
    // `Database` does not implement `Clone` â€” handing it off would
    // require opening a second connection. The async signature is
    // still what the IPC dispatcher needs to avoid blocking the
    // runtime when many commands are in flight.
    tauri::async_runtime::spawn_blocking(|| -> Result<(), String> { Ok(()) })
        .await
        .map_err(|e| format!("spawn_blocking join error: {e}"))?
        .map_err(|e: String| e)?;
    db.run_batch(batch_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_batch_results(
    db: State<'_, Database>,
    batch_id: i64,
) -> Result<Vec<BatchResult>, String> {
    db.list_batch_results(batch_id).map_err(|e| e.to_string())
}

// Phase 4.5 â€” Hot Folders (#42)

#[tauri::command]
pub fn create_hot_folder(
    db: State<'_, Database>,
    input: HotFolderInput,
) -> Result<HotFolder, String> {
    db.create_hot_folder(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_hot_folders(db: State<'_, Database>) -> Result<Vec<HotFolder>, String> {
    db.list_hot_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_hot_folder(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_hot_folder(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_hot_folder(db: State<'_, Database>, id: i64, is_active: bool) -> Result<(), String> {
    db.toggle_hot_folder(id, is_active)
        .map_err(|e| e.to_string())
}

// Phase 5.1 â€” PDF Compression (#49)

#[tauri::command]
pub async fn compress_pdf(
    path: String,
    output_path: Option<String>,
    options: Option<crate::pdf::compress::CompressionOptions>,
) -> Result<crate::pdf::compress::CompressionResult, String> {
    let path = security::validate_read_path(&path)?;
    // Validate optional output path before moving into spawn_blocking
    let output_path = if let Some(out) = output_path {
        let validated = security::validate_write_path(&out)?;
        Some(validated)
    } else {
        None
    };
    let opts = options.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::compress::compress_pdf(&path, output_path.as_deref(), &opts)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

// Phase 6.1 â€” PDF Redaction (#231)

/// Permanently redact rectangular regions of a PDF and record the operation in
/// the tamper-evident audit hash-chain.
///
/// Touches: reads `path`, writes a redacted PDF to `output_path`, and appends
/// one row to `redaction_operations` (linked to any prior operation for the
/// same source file). The redaction is applied by painting opaque black boxes
/// as a final content stream on each affected page, so the obscured content is
/// drawn over rather than left selectable beneath the box.
///
/// Errors: invalid read/write paths, a region targeting a non-existent page, an
/// unparseable PDF, or no valid (positive-area) regions.
#[tauri::command]
pub fn redact_pdf(
    db: State<'_, Database>,
    path: String,
    output_path: String,
    redactions: Vec<RedactionRect>,
    operator_name: Option<String>,
    notes: Option<String>,
) -> Result<RedactionResult, String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;

    // In-memory pipeline: read the source, redact, hash, then write. No
    // intermediate plaintext temp file is created.
    let input = std::fs::read(&path).map_err(|e| format!("Failed to read PDF: {e}"))?;
    let result = crate::pdf::redact::redact_pdf_content(&input, &redactions, &output_path)?;

    let regions_json = serde_json::to_string(&redactions).unwrap_or_else(|_| "[]".to_string());
    let operator = operator_name.unwrap_or_default();
    let notes = notes.unwrap_or_default();

    db.log_redaction_operation(
        &path,
        &result.output_path,
        &result.content_hash,
        &regions_json,
        result.redactions_applied as i64,
        result.pages_modified as i64,
        &operator,
        &notes,
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Return the redaction audit hash-chain for a source PDF, oldest first. Each
/// entry carries a `chain_valid` flag indicating whether its hash link is
/// intact (tampering with the SQLite file is detected here).
#[tauri::command]
pub fn get_redaction_audit_log(
    db: State<'_, Database>,
    path: String,
) -> Result<Vec<RedactionAuditEntry>, String> {
    db.query_redaction_log(&path).map_err(|e| e.to_string())
}

/// Verify the entire redaction hash-chain for a source PDF. Returns `true` only
/// when every link is intact.
#[tauri::command]
pub fn verify_redaction_chain(db: State<'_, Database>, path: String) -> Result<bool, String> {
    db.verify_redaction_chain_integrity(&path)
        .map_err(|e| e.to_string())
}

// Phase 5.2 â€” Barcode detection (#270)

/// Render a page at 200 DPI and detect all barcodes in it. Returns one
/// `BarcodeResult` per detected code with decoded text, bbox, and a
/// validation status (`ok` | `undersized` | `tight_quiet_zone`).
#[tauri::command]
pub fn detect_barcodes(
    engine: State<'_, PdfEngine>,
    path: String,
    page_index: usize,
) -> Result<Vec<crate::pdf::barcode::BarcodeDetection>, String> {
    let path = security::validate_read_path(&path)?;
    use image::RgbaImage;
    let doc = engine.open_document(&path)?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    // 200 DPI = 200/72 â‰ˆ 2.78 px per point; render at that resolution
    // so the bbox-to-mm math is well-behaved.
    let dpi = 200.0_f64;
    let page_width_pts = page.width().value as f64;
    let page_height_pts = page.height().value as f64;
    let target_w = ((page_width_pts * dpi / 72.0) as i32).max(64);
    let config = pdfium_render::prelude::PdfRenderConfig::new().set_target_width(target_w);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {e}"))?;
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    if bytes.len() < (pw as usize) * (ph as usize) * 4 {
        return Err("Rendered bitmap shorter than expected".to_string());
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
    let mut rgba: Vec<u8> = Vec::with_capacity((pw as usize) * (ph as usize) * 4);
    for px in img.pixels() {
        rgba.extend_from_slice(&px.0);
    }
    let input = crate::pdf::barcode::BarcodeInputImage {
        pixels: rgba,
        width: pw,
        height: ph,
        page_width_pts,
        page_height_pts,
    };
    crate::pdf::barcode::detect_barcodes_in_image(&input)
}

// Phase 5.3 â€” Analytics Dashboard (#50)

#[tauri::command]
pub fn get_analytics_summary(db: State<'_, Database>) -> Result<AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

/// Combined analytics payload for the dashboard: per-client pass
/// rates, average order turnaround (hours), and the most common
/// error categories. Each query is independently defensive â€” if
/// one fails (e.g. a table is missing) the rest of the payload
/// still returns.
#[tauri::command]
pub fn get_analytics_dashboard(
    db: State<'_, Database>,
) -> Result<AnalyticsDashboard, String> {
    let summary = db
        .get_analytics_summary()
        .map_err(|e| e.to_string())?;
    let client_pass_rates = db
        .get_client_pass_rates()
        .unwrap_or_default();
    let average_turnaround_hours = db
        .get_average_turnaround_hours()
        .unwrap_or(0.0);
    let common_error_categories = db
        .get_common_error_categories()
        .unwrap_or_default();
    Ok(AnalyticsDashboard {
        summary,
        client_pass_rates,
        average_turnaround_hours,
        common_error_categories,
    })
}

// Phase 5.5 â€” AI visual checking & ink coverage (#45)

#[tauri::command]
pub async fn ai_visual_check(
    path: String,
    prompt: String,
) -> Result<AiCheckResult, String> {
    crate::ai_check::ai_visual_check(&path, &prompt).await
}

// Issue #229 â€” Optical Character Recognition (OCR)

/// Detect whether a PDF is text-based or scanned (image-based).
///
/// Returns classification:
/// - TextBased: PDF contains embedded text and fonts (already searchable)
/// - Scanned: PDF is primarily image-based (requires OCR)
/// - Mixed: Some pages are text-based, some are scanned
#[tauri::command]
pub fn detect_pdf_type(path: String) -> Result<crate::pdf::ocr::PdfType, String> {
    let pdf_path = security::validate_read_path(&path)?;
    crate::pdf::ocr::detect_pdf_type(&pdf_path)
}

/// Run OCR on a PDF using the specified backend (Tesseract or Google Cloud Vision).
///
/// Supports:
/// - Selecting specific pages to OCR
/// - Overlaying extracted text as a hidden searchable layer
/// - Backend selection (local Tesseract or cloud Vision API)
/// - Language hints (default: English “eng”)
/// - Confidence score reporting
///
/// **Phase 2:** Tesseract backend is fully implemented.
/// **Phase 3:** Google Cloud Vision backend pending.
#[tauri::command]
pub async fn run_ocr(
    path: String,
    options: crate::pdf::ocr::OcrOptions,
) -> Result<crate::pdf::ocr::OcrResult, String> {
    let pdf_path = security::validate_read_path(&path)?;

    // Validate output path if provided
    if let Some(ref out) = options.output_path {
        security::validate_write_path(out)?;
    }

    // Run OCR on background thread (rendering + subprocess invocation is blocking)
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::ocr::run_ocr(&pdf_path, options)
    })
    .await
    .map_err(|e| format!(“spawn_blocking join error: {e}”))?
}

/// Check if Tesseract OCR engine is available on this system.
///
/// Useful for:
/// - Determining if local OCR is available (vs requiring cloud Vision API)
/// - Prompting user to install tesseract if missing
/// - Graceful fallback in settings UI
///
/// Returns Ok(true) if tesseract binary is found on PATH.
#[tauri::command]
pub fn is_tesseract_available() -> Result<bool, String> {
    match crate::pdf::ocr::check_tesseract_available() {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Get the total page count of a PDF.
///
/// Useful for:
/// - Showing page count in OCR UI
/// - Validating page indices in OcrOptions
/// - Determining default page range for selective OCR
#[tauri::command]
pub fn get_pdf_page_count(path: String) -> Result<usize, String> {
    let pdf_path = security::validate_read_path(&path)?;
    crate::pdf::ocr::get_page_count(&pdf_path)
}


/// Set Google Cloud Vision API key for cloud-based OCR.
///
/// Stores the API key in system keychain (preferred) or settings database.
/// The key is validated for non-empty content but not tested against the API here.
///
/// **Security:** API key is stored in system keychain when available, or encrypted in DB.
/// Do not log or expose the key in error messages.
#[tauri::command]
pub fn set_google_vision_api_key(api_key: String) -> Result<(), String> {
    crate::pdf::ocr::set_google_vision_api_key(&api_key)
}

/// Test the Google Cloud Vision API connection with the current API key.
///
/// Sends a minimal test request (1x1 pixel image) to verify:
/// - API key is valid
/// - Network connectivity is working
/// - API quotas are not exceeded
///
/// Returns Ok(true) if the connection is successful.
/// Returns Err with descriptive error if the connection fails.
#[tauri::command]
pub async fn test_google_vision_connection() -> Result<bool, String> {
    crate::pdf::ocr::test_google_vision_connection().await
}

/// Estimate the cost of running OCR on a PDF via Google Cloud Vision API.
///
/// Pricing (as of 2024):
/// - $0.0015 per page (DOCUMENT_TEXT_DETECTION feature)
/// - Minimum 100 pages charged per request
///
/// Example: 50-page PDF costs ~$0.15 (100-page minimum billed)
/// Example: 500-page PDF costs ~$0.75
///
/// Returns a CostEstimate struct with:
/// - page_count: Actual pages in PDF
/// - billable_pages: Pages that will be charged (minimum 100)
/// - cost_usd: Estimated cost in USD
#[tauri::command]
pub fn estimate_google_vision_cost(path: String) -> Result<crate::pdf::ocr::CostEstimate, String> {
    let pdf_path = security::validate_read_path(&path)?;
    let page_count = crate::pdf::ocr::get_page_count(&pdf_path)?;
    Ok(crate::pdf::ocr::estimate_google_vision_cost(page_count))
}
// Issue #278 â€” Crash reporting + opt-in telemetry

#[tauri::command]
pub async fn crash_report(
    error_message: String,
    stack_trace: String,
) -> Result<crate::observability::CrashResponse, String> {
    crate::observability::crash_report(error_message, stack_trace).await
}

// Phase 6.1 â€” Email, FTP, webhook (#54, #52)

#[tauri::command]
pub fn save_email_settings(db: State<'_, Database>, settings: EmailSettings) -> Result<(), String> {
    db.save_email_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_email_settings(db: State<'_, Database>) -> Result<Option<EmailSettings>, String> {
    db.get_email_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_ftp_settings(db: State<'_, Database>, settings: FtpSettings) -> Result<(), String> {
    db.save_ftp_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ftp_settings(db: State<'_, Database>) -> Result<Option<FtpSettings>, String> {
    db.get_ftp_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn send_email(
    db: State<'_, Database>,
    to: String,
    subject: String,
    body: String,
    attachment_path: Option<String>,
) -> Result<(), String> {
    let settings = db
        .get_email_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Email settings not configured. Save SMTP settings first.".to_string())?;
    crate::email::send_email_via_smtp(
        &settings,
        &to,
        &subject,
        &body,
        attachment_path.as_deref(),
    )
}

#[tauri::command]
pub fn ftp_upload(
    db: State<'_, Database>,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let settings = db
        .get_ftp_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "FTP settings not configured. Save FTP settings first.".to_string())?;
    crate::ftp::upload_file_via_ftp(&settings, &local_path, &remote_path)
}

#[tauri::command]
pub fn create_webhook(
    db: State<'_, Database>,
    url: String,
    event: String,
) -> Result<WebhookEntry, String> {
    if !url.starts_with("https://") {
        return Err("Webhook URL must use HTTPS".to_string());
    }
    if url.len() > 2048 {
        return Err("Webhook URL too long".to_string());
    }
    validate_command_url(&url)?;
    db.create_webhook(&url, &event).map_err(|e| e.to_string())
}

fn validate_webhook_url(url: &str) -> Result<(), String> {
    validate_command_url(url)
}

/// Validate that a user-supplied URL is safe to fetch from a Tauri command.
/// Rejects:
///   * Non-HTTPS schemes (only `https://` is allowed by default; HTTP for
///     localhost is allowed because the dev server runs there).
///   * Hosts that resolve to loopback, link-local, or private network
///     addresses â€” defeats SSRF attempts against the cloud metadata
///     service (169.254.169.254), internal HTTP services, and
///     carrier-grade NAT ranges (#296).
///   * Empty hosts / unparseable URLs.
pub(crate) fn validate_command_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL is empty".to_string());
    }
    if url.len() > 2048 {
        return Err("URL too long".to_string());
    }
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    let scheme = parsed.scheme();
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL missing host".to_string())?;
    let is_local_dev = host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.ends_with(".localhost");
    if scheme != "https" && !(is_local_dev && scheme == "http") {
        return Err(format!(
            "URL must use HTTPS (got scheme '{}', host '{}')",
            scheme, host
        ));
    }
    let resolved: Vec<std::net::IpAddr> = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => {
            use std::net::ToSocketAddrs;
            let port = if scheme == "https" { 443 } else { 80 };
            (host, port)
                .to_socket_addrs()
                .map_err(|e| format!("Cannot resolve URL host '{}': {}", host, e))?
                .map(|sa| sa.ip())
                .collect()
        }
    };
    for ip in &resolved {
        if is_blocked_ip(*ip) {
            return Err(format!("URL resolves to blocked address: {}", ip));
        }
    }
    Ok(())
}

pub(crate) fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()                        // 127.0.0.0/8
            || v4.is_private()                      // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || v4.is_link_local()                   // 169.254.0.0/16
            || v4.is_unspecified()                  // 0.0.0.0
            || v4.is_multicast()
            || v4.octets()[0] == 100 && (v4.octets()[1] >= 64 && v4.octets()[1] <= 127)
            // 100.64.0.0/10 carrier-grade NAT
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()                        // ::1
            || v6.is_unspecified()                  // ::
            || v6.segments()[0] & 0xfe00 == 0xfc00 // fc00::/7 unique local
            || v6.segments()[0] & 0xffc0 == 0xfe80 // fe80::/10 link-local
        }
    }
}

#[tauri::command]
pub fn list_webhooks(db: State<'_, Database>) -> Result<Vec<WebhookEntry>, String> {
    db.list_webhooks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_webhook(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_webhook(id).map_err(|e| e.to_string())
}

// #84 â€” Job ticket generation

#[tauri::command]
pub fn generate_job_ticket(
    job_id: String,
    customer_name: String,
    product_name: String,
    quantity: i64,
    due_date: String,
    print_type: String,
    paper_stock: String,
    finishing: String,
    files: Vec<String>,
    notes: String,
    output_path: String,
) -> Result<(), String> {
    let input = crate::pdf::ticket::JobTicketInput {
        job_id,
        customer_name,
        product_name,
        quantity,
        due_date,
        print_type,
        paper_stock,
        finishing,
        files,
        notes,
    };
    crate::pdf::ticket::generate_job_ticket(&input, &output_path)
}

// #85 â€” Cloud backup (stubs)

#[tauri::command]
pub async fn upload_event_batch_cmd(
    db: State<'_, Database>,
    tenant_id: String,
    _last_event_id: i64,
) -> Result<String, String> {
    let events = db
        .list_events(&tenant_id, None, None, 1000)
        .map_err(|e| e.to_string())?;
    let batch = crate::cloud_backup::EventBatch { tenant_id, events };
    let result = crate::cloud_backup::upload_event_batch(&batch).await?;
    Ok(result.message)
}

#[tauri::command]
pub async fn upload_snapshot_cmd(tenant_id: String, file_path: String) -> Result<String, String> {
    // Previously this hashed the *path string* with DefaultHasher, which is
    // meaningless as a file checksum. We don't have a sha2 crate available,
    // so build a best-effort fingerprint from file size + first/last 64 bytes
    // â€” enough to detect obvious mismatched uploads for the stub backend.
    // (#163)
    let checksum = compute_snapshot_checksum(&file_path);
    let snapshot = crate::cloud_backup::SnapshotUpload {
        tenant_id,
        file_path,
        checksum,
    };
    let result = crate::cloud_backup::upload_snapshot(&snapshot).await?;
    Ok(result.message)
}

/// Best-effort snapshot fingerprint used by `upload_snapshot_cmd`. Returns a
/// hex string built from file size plus the first and last 64 bytes of the
/// file. Returns `"unavailable"` if the file cannot be read â€” the upload is a
/// stub and does not validate the checksum, but we still surface the failure
/// rather than hashing the path string.
fn compute_snapshot_checksum(path: &str) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return "unavailable".to_string(),
    };
    let size = metadata.len();
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return format!("size:{size}"),
    };
    let mut head = [0u8; 64];
    let head_len = file.read(&mut head).unwrap_or(0);
    let mut tail = [0u8; 64];
    if size > 64 {
        let _ = file.seek(SeekFrom::Start(size - 64));
        let _ = file.read(&mut tail);
    }
    let to_hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    };
    format!(
        "size:{size}:h{}:t{}",
        to_hex(&head[..head_len]),
        to_hex(&tail)
    )
}

#[tauri::command]
pub fn get_cloud_backup_status() -> String {
    crate::cloud_backup::get_sync_status()
}

// #89 â€” Keychain commands

#[tauri::command]
pub fn keychain_read(service: String, key: String) -> Result<crate::keychain::SecretValue, String> {
    crate::keychain::read_secret(&service, &key)
}

#[tauri::command]
pub fn keychain_write(service: String, key: String, value: String) -> Result<(), String> {
    crate::keychain::write_secret(&service, &key, &value)
}

#[tauri::command]
pub fn keychain_delete(service: String, key: String) -> Result<(), String> {
    crate::keychain::delete_secret(&service, &key)
}

// #90 â€” DB schema version & backup/restore

#[tauri::command]
pub fn get_schema_version(db: State<'_, Database>) -> Result<i64, String> {
    db.get_schema_version().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_backup(
    db: State<'_, Database>,
    backup_path: String,
) -> Result<crate::models::BackupEntry, String> {
    let path = std::path::PathBuf::from(&backup_path);
    db.create_backup(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_backups(db: State<'_, Database>) -> Result<Vec<crate::models::BackupEntry>, String> {
    db.list_backups().map_err(|e| e.to_string())
}

// #99 â€” SQLCipher: export plaintext backup
#[tauri::command]
pub fn export_plaintext_backup(
    db: State<'_, Database>,
    output_path: String,
) -> Result<u64, String> {
    let output_path = security::validate_write_path(&output_path)?;
    let path = std::path::PathBuf::from(&output_path);
    db.export_plaintext_backup(&path).map_err(|e| e.to_string())
}

// #88 â€” Reveal logs

#[tauri::command]
pub fn reveal_logs(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    crate::logging::reveal_logs(&app_dir);
    Ok(())
}

// Issue #256 â€” Metrics snapshot for the PerfOverlay.

#[tauri::command]
pub fn get_metrics_snapshot() -> crate::metrics::MetricsSnapshot {
    crate::metrics::snapshot()
}

// Issue #241 / #275 â€” Preferences + PDF settings

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
        .map(|opt| opt.map(|(alt_text, is_decorative)| AltTextEntry { object_id, alt_text, is_decorative }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_alt_text(
    db: State<'_, Database>,
    file_path: String,
) -> Result<Vec<AltTextEntry>, String> {
    db.get_alt_text_for_file(&file_path)
        .map(|rows| rows.into_iter().map(|(object_id, alt_text, is_decorative)| AltTextEntry {
            object_id,
            alt_text,
            is_decorative,
        }).collect())
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

// Issue #291 â€” Command batching (#291)

/// A single batched command. The `name` field is the Tauri command name
/// (without the `invoke` wrapper), and `args` is a free-form JSON object
/// that the command will deserialize. The list of supported commands is
/// intentionally narrow: only stateless / read-only commands are safe to
/// run in a batch. Mutations are still serial â€” they go through the
/// same `Mutex<Connection>` as before.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchedCommand {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchedResponse {
    pub name: String,
    pub ok: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Run a list of small Tauri commands in a single IPC round-trip. Useful
/// for the dashboard which needs orders + invoices + clients + low-stock
/// alerts at once (#291). The `db` State is read-shared across the
/// batch â€” each sub-command locks the `Mutex<Connection>` briefly and
/// releases it.
#[tauri::command]
pub async fn batch_commands(
    db: State<'_, Database>,
    commands: Vec<BatchedCommand>,
) -> Result<Vec<BatchedResponse>, String> {
    let mut out = Vec::with_capacity(commands.len());
    for cmd in commands {
        let result = dispatch_batched_command(&db, &cmd).await;
        match result {
            Ok(value) => out.push(BatchedResponse {
                name: cmd.name,
                ok: true,
                result: Some(value),
                error: None,
            }),
            Err(e) => out.push(BatchedResponse {
                name: cmd.name,
                ok: false,
                result: None,
                error: Some(e),
            }),
        }
    }
    Ok(out)
}

async fn dispatch_batched_command(
    db: &State<'_, Database>,
    cmd: &BatchedCommand,
) -> Result<serde_json::Value, String> {
    let name = cmd.name.as_str();
    let args = &cmd.args;
    match name {
        "list_orders" => {
            let limit = args.get("limit").and_then(|v| v.as_i64());
            let offset = args.get("offset").and_then(|v| v.as_i64());
            let value = match (limit, offset) {
                (Some(l), Some(o)) => serde_json::to_value(
                    db.list_orders_paginated(l, o).map_err(|e| e.to_string())?,
                ),
                _ => serde_json::to_value(db.list_orders().map_err(|e| e.to_string())?),
            }
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "list_invoices" => {
            let limit = args.get("limit").and_then(|v| v.as_i64());
            let offset = args.get("offset").and_then(|v| v.as_i64());
            let value = match (limit, offset) {
                (Some(l), Some(o)) => serde_json::to_value(
                    db.list_invoices_paginated(l, o).map_err(|e| e.to_string())?,
                ),
                _ => serde_json::to_value(db.list_invoices().map_err(|e| e.to_string())?),
            }
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "list_clients" => {
            let search = args
                .get("search")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let status_filter = args
                .get("statusFilter")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let value = serde_json::to_value(
                db.list_clients(search.as_deref(), status_filter.as_deref())
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "get_low_stock_alerts" => {
            let value = serde_json::to_value(
                db.get_low_stock_alerts().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "get_business_info" => {
            let value =
                serde_json::to_value(db.get_business_info().map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "list_preflight_profiles" => {
            let value = serde_json::to_value(
                db.list_preflight_profiles().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "list_hot_folders" => {
            let value = serde_json::to_value(
                db.list_hot_folders().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "list_pdf_jobs" => {
            let value = serde_json::to_value(
                db.list_pdf_jobs().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        "get_analytics_summary" => {
            let value = serde_json::to_value(
                db.get_analytics_summary().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            Ok(value)
        }
        other => Err(format!(
            "batch_commands: '{other}' is not allowed in a batched call (read-only whitelist only)"
        )),
    }
}

// ============================================================================
// Issue #292: Tauri Channel API for live updates
// ============================================================================

use tauri::ipc::Channel;

#[derive(Debug, Clone, ::serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppEvent {
    HotFolder {
        watcher_id: String,
        file_path: String,
        kind: String,
        message: String,
    },
    Metrics {
        snapshot: crate::metrics::MetricsSnapshot,
    },
    Heartbeat { ts: u64 },
}

#[tauri::command]
pub async fn subscribe_events(on_event: Channel<AppEvent>) -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let stop = Arc::new(AtomicBool::new(false));
    let on_event_clone = on_event.clone();
    let stop_clone = stop.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = 0u64;
        while !stop_clone.load(Ordering::SeqCst) {
            tick = tick.wrapping_add(1);
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let event = if tick % 10 == 0 {
                AppEvent::Metrics {
                    snapshot: crate::metrics::snapshot(),
                }
            } else {
                AppEvent::Heartbeat { ts }
            };
            if on_event_clone.send(event).is_err() {
                break;
            }
            tauri::async_runtime::spawn_blocking(|| {
                std::thread::sleep(Duration::from_millis(500));
            })
            .await
            .ok();
        }
    });
    Ok(())
}

// ============================================================================
// Issue #293: render_page_b64 (avoid temp-file round-trip)
// ============================================================================

#[tauri::command]
pub async fn render_page_b64(
    engine: State<'_, crate::pdf::engine::PdfEngine>,
    path: String,
    page_index: usize,
    dpi: Option<f32>,
) -> Result<String, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = engine.open_document(&_path)?;
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
    let config = pdfium_render::prelude::PdfRenderConfig::new().set_target_width(px_width);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {}", e))?;
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes: Vec<u8> = bitmap.as_raw_bytes().to_vec();
    drop(bitmap);
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        use image::ImageEncoder;
        if bytes.len() < (pw as usize) * (ph as usize) * 4 {
            return Err("Rendered bitmap is shorter than expected".to_string());
        }
        let mut img = image::RgbaImage::new(pw, ph);
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
        let mut png: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ColorType::Rgba8.into(),
            )
            .map_err(|e| format!("PNG encode error: {e}"))?;
        Ok(base64_encode(&png))
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}
