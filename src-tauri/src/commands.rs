use std::path::PathBuf;

use lopdf::Object;
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
use crate::pdf::security::SecurityFinding;
use crate::pdf::transforms::{ConversionResult, IccProfileInfo};

fn validate_read_path(path: &str) -> Result<PathBuf, String> {
    if path.contains('\0') {
        return Err("Path contains null bytes".to_string());
    }
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(format!("File not found: {}", path));
    }
    p.canonicalize().map_err(|e| format!("Invalid path: {}", e))
}

/// Validate a path used as an `output_path` in a Tauri command. The path
/// must:
///   1. Contain no NUL bytes.
///   2. Canonicalize to a non-empty absolute path whose parent directory
///      already exists. (We don't require the file itself to exist; we're
///      about to write it.)
///   3. Not be inside a system / read-only location that we know we should
///      never write user data to.
///   4. Not contain a parent-traversal (`..`) component after canonicalization
///      relative to its original form.
fn validate_write_path(path: &str) -> Result<PathBuf, String> {
    if path.contains('\0') {
        return Err("Output path contains null bytes".to_string());
    }
    if path.is_empty() {
        return Err("Output path is empty".to_string());
    }
    let p = PathBuf::from(path);
    let parent = p
        .parent()
        .ok_or_else(|| "Output path has no parent directory".to_string())?;
    if !parent.exists() {
        return Err(format!(
            "Output parent directory does not exist: {}",
            parent.display()
        ));
    }
    // Reject system locations on Windows and Unix.
    #[cfg(windows)]
    {
        let s = parent.to_string_lossy().to_lowercase();
        for blocked in [
            "c:\\windows",
            "c:\\program files",
            "c:\\program files (x86)",
            "c:\\programdata",
        ] {
            if s.starts_with(blocked) {
                return Err(format!(
                    "Output path is inside a system location: {}",
                    parent.display()
                ));
            }
        }
    }
    #[cfg(unix)]
    {
        let s = parent.to_string_lossy();
        for blocked in [
            "/etc", "/usr", "/bin", "/sbin", "/var", "/boot", "/sys", "/proc", "/root",
        ] {
            if s == blocked || s.starts_with(&format!("{}/", blocked)) {
                return Err(format!(
                    "Output path is inside a system location: {}",
                    parent.display()
                ));
            }
        }
    }
    // Reject explicit traversal in the original path string.
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Output path contains '..'".to_string());
        }
    }
    // Canonicalize the PARENT directory (which must exist) and re-join the
    // filename. We cannot canonicalize the full path because the output file
    // doesn't exist yet — std::fs::canonicalize requires the path to exist.
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize output directory: {}", e))?;
    let file_name = p
        .file_name()
        .ok_or_else(|| "Output path has no filename component".to_string())?;
    Ok(canonical_parent.join(file_name))
}

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
    let _ = validate_read_path(&file_path)?;
    let path = PathBuf::from(&file_path);
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
    let _ = validate_read_path(&file_path)?;
    let path = PathBuf::from(&file_path);
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
    let _ = validate_read_path(&path)?;
    let p = PathBuf::from(&path);
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

// ── Clients ───────────────────────────────────────────────────────────────────

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

// ── Art Approvals ─────────────────────────────────────────────────────────────

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

// ── Payments (#10, #11) ───────────────────────────────────────────────────────

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

// ── Invoice reminders (#9) ────────────────────────────────────────────────────

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

// ── QuickBooks sync (#7) ──────────────────────────────────────────────────────

#[tauri::command]
pub fn update_invoice_qb_status(
    db: State<'_, Database>,
    id: i64,
    status: String,
) -> Result<(), String> {
    db.update_invoice_qb_status(id, &status)
        .map_err(|e| e.to_string())
}

// ── Job specs + production + fulfillment (#15, #16, #18) ─────────────────────

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

// ── Department notes (#18) ────────────────────────────────────────────────────

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
    let _ = validate_read_path(&path)?;
    let path_buf = PathBuf::from(&path);
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
    let _ = validate_read_path(&file_path)?;
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
    let _ = validate_read_path(&path)?;
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
    let _ = validate_read_path(&path)?;
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
    let _ = validate_read_path(&path)?;
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
    let _ = validate_read_path(&path)?;
    let doc = engine.open_document(&path)?;
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
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
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
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
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
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
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
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::fonts::collect_fonts(&doc))
}

#[tauri::command]
pub fn check_page_boxes(path: String) -> Result<Vec<PageBoxFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::boxes::check_page_boxes(&doc))
}

#[tauri::command]
pub fn check_image_resolution(path: String) -> Result<Vec<ImageResolutionFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::images::check_image_resolution(&doc))
}

#[tauri::command]
pub fn check_bleed(path: String, min_bleed_mm: Option<f64>) -> Result<Vec<BleedFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let min = min_bleed_mm.unwrap_or(3.0);
    Ok(crate::pdf::bleed::check_bleed(&doc, min))
}

#[tauri::command]
pub fn add_bleed(path: String, amount_mm: f64, output_path: String) -> Result<(), String> {
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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

    for obj_id in &page_ids {
        let page_dict = doc
            .get_dictionary_mut(*obj_id)
            .map_err(|e| format!("Failed to get page dict: {}", e))?;

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

        // Expand MediaBox if needed
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
    }

    doc.save(&output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn check_output_intents(path: String) -> Result<Vec<OutputIntent>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::metadata::get_output_intents(&doc))
}

#[tauri::command]
pub fn check_security(path: String) -> Result<Vec<SecurityFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;

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
            message: "PDF/X-1a does not support live transparency. If the file contains transparent objects, they must be flattened. This check is a stub — manual verification recommended.".into(),
            fix_hint: "In InDesign: export with PDF/X-1a preset (handles flattening). In Illustrator: flatten transparency in Object → Flatten Transparency before exporting.".into(),
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
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_color_spaces(&doc, &target_profile))
}

#[tauri::command]
pub fn check_overprint(path: String) -> Result<Vec<OverprintFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_overprint(&doc))
}

#[tauri::command]
pub fn check_transparency(path: String) -> Result<Vec<TransparencyFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_transparency(&doc))
}

#[tauri::command]
pub fn check_hidden_content(path: String) -> Result<Vec<HiddenContentFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_hidden_content(&doc))
}

#[tauri::command]
pub fn check_spot_colors(path: String) -> Result<Vec<SpotColorFinding>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_spot_colors(&doc))
}

#[tauri::command]
pub fn check_ink_coverage(path: String) -> Result<Vec<InkCoverageFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_ink_coverage(&doc))
}

#[tauri::command]
pub fn list_icc_profiles() -> Vec<IccProfileInfo> {
    crate::pdf::transforms::get_bundled_icc_profiles()
}

#[tauri::command]
#[allow(unused_variables)]
pub fn convert_rgb_to_cmyk(
    path: String,
    output_path: String,
    scope: Option<String>,
    src_profile: Option<String>,
    dst_profile: Option<String>,
    rendering_intent: Option<String>,
) -> Result<ConversionResult, String> {
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let scope = scope.as_deref().unwrap_or("both");
    let result = crate::pdf::transforms::convert_rgb_to_cmyk(&mut doc, scope)?;
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save converted PDF: {}", e))?;
    Ok(result)
}

#[tauri::command]
pub fn add_output_intent(
    path: String,
    output_path: String,
    icc_profile: String,
    condition_id: String,
    condition: String,
) -> Result<(), String> {
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    // The ICC profile file path is passed; read it
    let icc_data =
        std::fs::read(&icc_profile).map_err(|e| format!("Failed to read ICC profile: {}", e))?;
    crate::pdf::transforms::add_output_intent(&mut doc, &icc_data, &condition_id, &condition)?;
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_pdf_catalog(path: String) -> Result<serde_json::Value, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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

// ── Preflight findings persistence (Days 43-44) ────────────────────────────

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

// ═══════════════════════════════════════════════════════════════════════════
// Phase 3.2 — Layers & page operations
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn reorder_pages(
    path: String,
    new_order: Vec<usize>,
    output_path: String,
) -> Result<(), String> {
    use lopdf::Object;
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
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
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
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
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
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
                layers.push(LayerInfo {
                    name,
                    visible: true,
                    locked: false,
                    object_id: obj_id.0,
                });
            }
        }
    }
    Ok(layers)
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 3.3 — Content-stream round-trip (#91)
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn decode_content_stream(path: String, page_index: usize) -> Result<String, String> {
    use crate::pdf::content_stream;
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
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
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
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

// ═══════════════════════════════════════════════════════════════════════════
// Phase 3.4 — Text search & replacement (#31)
// ═══════════════════════════════════════════════════════════════════════════

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
pub fn search_text(path: String, query: String) -> Result<Vec<TextMatch>, String> {
    let _ = validate_read_path(&path)?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {e}"))?;
    let page_count = doc.get_pages().len();
    let mut results = Vec::new();
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
            results.push(TextMatch {
                page_index,
                text: snippet,
                char_index: abs_pos,
                length: query.len(),
                bbox: None,
            });
            // Advance past the matched query (#164) — previously advanced by
            // 1 char, producing overlapping matches and O(n²) scan cost.
            start = abs_pos + lower_query.len();
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn replace_text(
    path: String,
    page_index: usize,
    find: String,
    replace: String,
    output_path: String,
) -> Result<ReplaceResult, String> {
    if find.is_empty() {
        return Err("`find` string must not be empty".to_string());
    }
    let content = decode_content_stream(path.clone(), page_index)?;
    let replacements = content.matches(find.as_str()).count();
    let new_content = content.replace(find.as_str(), &replace);
    encode_content_stream(path.clone(), page_index, new_content, output_path.clone())?;
    Ok(ReplaceResult {
        replacements_made: replacements,
        output_path,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 3.5 — Image replacement & optimization (#32)
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn replace_image(
    path: String,
    _page_index: usize,
    _xobject_name: String,
    _new_image_path: String,
    output_path: String,
) -> Result<(), String> {
    let _ = validate_write_path(&output_path)?;
    tracing::warn!("replace_image is a stub — image replacement not yet implemented");
    std::fs::copy(&path, &output_path).map_err(|e| format!("Failed to copy PDF: {e}"))?;
    Ok(())
}

#[tauri::command]
#[allow(unused_variables)]
pub fn optimize_image(
    path: String,
    page_index: usize,
    xobject_name: String,
    settings: OptimizeSettings,
    output_path: String,
) -> Result<(), String> {
    let _ = validate_write_path(&output_path)?;
    tracing::warn!("optimize_image is a stub — image optimization not yet implemented");
    std::fs::copy(&path, &output_path).map_err(|e| format!("Copy failed: {e}"))?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 4.1 — Preflight Profiles (#39)
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn start_hot_folder_watcher(
    config: crate::pdf::watcher::HotFolderConfig,
) -> Result<(), String> {
    crate::pdf::watcher::start_hot_folder_watcher(&config)
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
pub fn run_profile(
    db: State<'_, Database>,
    profile_id: i64,
    path: String,
) -> Result<crate::pdf::registry::RunProfileResult, String> {
    let profile = db.get_preflight_profile(profile_id).map_err(|e| e.to_string())?;
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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
pub fn export_preflight_report_csv(
    db: State<'_, Database>,
    run_id: i64,
) -> Result<String, String> {
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

// ═══════════════════════════════════════════════════════════════════════════
// Phase 4.2 — Action Lists (#38)
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// Phase 4.3 — Batch Processing (#40)
// ═══════════════════════════════════════════════════════════════════════════

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
pub fn run_batch(db: State<'_, Database>, batch_id: i64) -> Result<(), String> {
    db.run_batch(batch_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_batch_results(
    db: State<'_, Database>,
    batch_id: i64,
) -> Result<Vec<BatchResult>, String> {
    db.list_batch_results(batch_id).map_err(|e| e.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 4.5 — Hot Folders (#42)
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// Phase 5.1 — PDF Compression (#49)
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn compress_pdf(path: String, output_path: String) -> Result<(), String> {
    let _ = validate_read_path(&path)?;
    let _ = validate_write_path(&output_path)?;
    crate::pdf::compress::compress_pdf(
        &path,
        &output_path,
        &crate::pdf::compress::CompressionOptions::default(),
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 5.2 — Barcode detection (#48)
// ═══════════════════════════════════════════════════════════════════════════

// Stub — actual zxing integration would go here
#[tauri::command]
pub fn detect_barcodes(_path: String) -> Result<Vec<BarcodeResult>, String> {
    Err("detect_barcodes is not implemented. Tracked in v2 polish issue #135.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 5.3 — Analytics Dashboard (#50)
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_analytics_summary(db: State<'_, Database>) -> Result<AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 5.5 — AI visual checking & ink coverage (#45)
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn ai_visual_check(_path: String, _prompt: String) -> Result<String, String> {
    Err("ai_visual_check is not implemented. Tracked in v2 polish issue #135.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 6.1 — Email, FTP, webhook (#54, #52)
// ═══════════════════════════════════════════════════════════════════════════

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
    validate_webhook_url(&url)?;
    db.create_webhook(&url, &event).map_err(|e| e.to_string())
}

fn validate_webhook_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid webhook URL: {}", e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "Webhook URL missing host".to_string())?;
    let resolved: Vec<std::net::IpAddr> = match host.parse::<std::net::IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => {
            use std::net::ToSocketAddrs;
            (host, 443)
                .to_socket_addrs()
                .map_err(|e| format!("Cannot resolve webhook host: {}", e))?
                .map(|sa| sa.ip())
                .collect()
        }
    };
    for ip in &resolved {
        if is_blocked_ip(*ip) {
            return Err(format!("Webhook URL resolves to blocked address: {}", ip));
        }
    }
    Ok(())
}

fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
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

// ═══════════════════════════════════════════════════════════════════════════
// #84 — Job ticket generation
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// #85 — Cloud backup (stubs)
// ═══════════════════════════════════════════════════════════════════════════

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
    // — enough to detect obvious mismatched uploads for the stub backend.
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
/// file. Returns `"unavailable"` if the file cannot be read — the upload is a
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

// ═══════════════════════════════════════════════════════════════════════════
// #89 — Keychain commands
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// #90 — DB schema version & backup/restore
// ═══════════════════════════════════════════════════════════════════════════

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

// #99 — SQLCipher: export plaintext backup
#[tauri::command]
pub fn export_plaintext_backup(
    db: State<'_, Database>,
    output_path: String,
) -> Result<u64, String> {
    let _ = validate_write_path(&output_path)?;
    let path = std::path::PathBuf::from(&output_path);
    db.export_plaintext_backup(&path).map_err(|e| e.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// #88 — Reveal logs
// ═══════════════════════════════════════════════════════════════════════════

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
