use std::path::PathBuf;

use std::io::{Read, BufReader};
use lopdf::Object;
use tauri::State;

use crate::cloud_import;
use crate::db::{Database, VerificationResult};
use crate::models::{*, BusinessInfo};
use crate::pdf::engine::PdfEngine;
use crate::pdf::boxes::PageBoxFinding;
use crate::pdf::fonts::FontFinding;
use crate::pdf::images::ImageResolutionFinding;
use crate::pdf::bleed::BleedFinding;
use crate::pdf::metadata::OutputIntent;
use crate::pdf::security::SecurityFinding;
use crate::pdf::color::{ColorSpaceFinding, SpotColorFinding, InkCoverageFinding};
use crate::pdf::transforms::IccProfileInfo;
use crate::pdf::overprint::{OverprintFinding, TransparencyFinding, HiddenContentFinding};
use crate::pdf::pdfx::PdfXFinding;

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
    db.create_invoice(&invoice_number, &due_date, &payment_terms).map_err(|e| {
        if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
            format!("Invoice number '{}' is already in use. Choose a different number.", invoice_number)
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
pub fn get_invoice(db: State<'_, Database>, id: i64) -> Result<InvoiceData, String> {
    db.get_invoice_data(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_invoice_line_item(db: State<'_, Database>, invoice_id: i64, description: String, quantity: f64, unit_price: f64) -> Result<InvoiceLineItem, String> {
    db.add_line_item(invoice_id, &description, quantity, unit_price).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct InvoiceLineItemInput {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
}

#[tauri::command]
pub fn replace_invoice_line_items(db: State<'_, Database>, invoice_id: i64, items: Vec<InvoiceLineItemInput>) -> Result<(), String> {
    let items_data: Vec<(String, f64, f64)> = items.into_iter()
        .map(|i| (i.description, i.quantity, i.unit_price))
        .collect();
    db.replace_invoice_line_items(invoice_id, &items_data).map_err(|e| e.to_string())
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
    db.create_estimate(&estimate_number, &valid_until).map_err(|e| {
        if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
            format!("Estimate number '{}' is already in use. Choose a different number.", estimate_number)
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
pub fn add_estimate_line_item(db: State<'_, Database>, estimate_id: i64, description: String, category: String, quantity: f64, unit_price: f64) -> Result<EstimateLineItem, String> {
    db.add_estimate_line_item(estimate_id, &description, &category, quantity, unit_price).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct EstimateLineItemInput {
    pub description: String,
    pub category: String,
    pub quantity: f64,
    pub unit_price: f64,
}

#[tauri::command]
pub fn replace_estimate_line_items(db: State<'_, Database>, estimate_id: i64, items: Vec<EstimateLineItemInput>) -> Result<(), String> {
    let items_data: Vec<(String, String, f64, f64)> = items.into_iter()
        .map(|i| (i.description, i.category, i.quantity, i.unit_price))
        .collect();
    db.replace_estimate_line_items(estimate_id, &items_data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_estimate(db: State<'_, Database>, id: i64, status: String, subtotal: f64, tax_rate: f64, tax_amount: f64, total: f64, notes: String, artwork_requirements: String) -> Result<(), String> {
    db.update_estimate(id, &status, subtotal, tax_rate, tax_amount, total, &notes, &artwork_requirements).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_inventory_item(db: State<'_, Database>, material_type: String, size: String, attributes: String, quantity: f64, unit: String, reorder_level: f64, alert_type: String, alert_threshold: f64) -> Result<InventoryItem, String> {
    db.add_inventory_item(&material_type, &size, &attributes, quantity, &unit, reorder_level, &alert_type, alert_threshold).map_err(|e| e.to_string())
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
pub fn adjust_inventory(db: State<'_, Database>, inventory_item_id: i64, quantity_change: f64, reason: String, order_id: Option<i64>) -> Result<(), String> {
    db.adjust_inventory(inventory_item_id, quantity_change, &reason, order_id).map_err(|e| e.to_string())
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
pub fn create_client(db: State<'_, Database>, name: String, company: String, email: String, phone: String, address: String, tags: String) -> Result<Client, String> {
    db.create_client(&name, &company, &email, &phone, &address, &tags).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_clients(db: State<'_, Database>, search: Option<String>, status_filter: Option<String>) -> Result<Vec<Client>, String> {
    db.list_clients(search.as_deref(), status_filter.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_client(db: State<'_, Database>, id: i64) -> Result<Client, String> {
    db.get_client(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_client(db: State<'_, Database>, id: i64, name: String, company: String, email: String, phone: String, address: String, tags: String, status: String, notes: String) -> Result<(), String> {
    db.update_client(id, &name, &company, &email, &phone, &address, &tags, &status, &notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_client(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_client(id).map_err(|e| e.to_string())
}

// ── Art Approvals ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_art_approval(db: State<'_, Database>, order_id: i64, file_path: String, staff_notes: String, follow_up_hours: i64) -> Result<ArtApproval, String> {
    db.create_art_approval(order_id, &file_path, &staff_notes, follow_up_hours).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_art_approvals_for_order(db: State<'_, Database>, order_id: i64) -> Result<Vec<ArtApproval>, String> {
    db.get_art_approvals_for_order(order_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn respond_to_art_approval(db: State<'_, Database>, token: String, status: String, customer_notes: String) -> Result<ArtApproval, String> {
    db.respond_to_art_approval(&token, &status, &customer_notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn increment_art_approval_follow_up(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.increment_art_approval_follow_up(id).map_err(|e| e.to_string())
}

// ── Payments (#10, #11) ───────────────────────────────────────────────────────

#[tauri::command]
pub fn record_payment(db: State<'_, Database>, invoice_id: Option<i64>, order_id: Option<i64>, amount: f64, payment_method: String, reference: String, notes: String) -> Result<Payment, String> {
    db.record_payment(invoice_id, order_id, amount, &payment_method, &reference, &notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_payments(db: State<'_, Database>, invoice_id: Option<i64>, order_id: Option<i64>) -> Result<Vec<Payment>, String> {
    db.list_payments(invoice_id, order_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_payment(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_payment(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_invoices_and_orders(db: State<'_, Database>, query: String) -> Result<Vec<serde_json::Value>, String> {
    db.search_invoices_and_orders(&query).map_err(|e| e.to_string())
}

// ── Invoice reminders (#9) ────────────────────────────────────────────────────

#[tauri::command]
pub fn log_invoice_reminder(db: State<'_, Database>, invoice_id: i64, method: String, notes: String) -> Result<InvoiceReminder, String> {
    db.log_invoice_reminder(invoice_id, &method, &notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_invoice_reminders(db: State<'_, Database>, invoice_id: i64) -> Result<Vec<InvoiceReminder>, String> {
    db.list_invoice_reminders(invoice_id).map_err(|e| e.to_string())
}

// ── QuickBooks sync (#7) ──────────────────────────────────────────────────────

#[tauri::command]
pub fn update_invoice_qb_status(db: State<'_, Database>, id: i64, status: String) -> Result<(), String> {
    db.update_invoice_qb_status(id, &status).map_err(|e| e.to_string())
}

// ── Job specs + production + fulfillment (#15, #16, #18) ─────────────────────

#[tauri::command]
pub fn update_order_job_specs(db: State<'_, Database>, id: i64, print_type: String, paper_stock: String, ink_colors: String, finishing: String, quantity: i64, production_notes: String, assigned_operator: String) -> Result<(), String> {
    db.update_order_job_specs(id, &print_type, &paper_stock, &ink_colors, &finishing, quantity, &production_notes, &assigned_operator).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order_fulfillment(db: State<'_, Database>, id: i64, fulfillment_method: String, tracking_number: String, tracking_carrier: String, ready_for_pickup: bool, shipped_at: Option<String>) -> Result<(), String> {
    db.update_order_fulfillment(id, &fulfillment_method, &tracking_number, &tracking_carrier, ready_for_pickup, shipped_at.as_deref()).map_err(|e| e.to_string())
}

// ── Department notes (#18) ────────────────────────────────────────────────────

#[tauri::command]
pub fn add_department_note(db: State<'_, Database>, order_id: i64, note: String, department: String) -> Result<DepartmentNote, String> {
    db.add_department_note(order_id, &note, &department).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_department_notes(db: State<'_, Database>, order_id: i64) -> Result<Vec<DepartmentNote>, String> {
    db.list_department_notes(order_id).map_err(|e| e.to_string())
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
            Object::String(s, _) => Some(String::from_utf8_lossy(s).to_string()),
            Object::Name(n) => Some(String::from_utf8_lossy(n).to_string()),
            _ => None,
        }
    })()
    .unwrap_or_default()
}

#[tauri::command]
pub fn open_pdf(engine: State<'_, PdfEngine>, path: String) -> Result<PdfSummary, String> {
    let path_buf = PathBuf::from(&path);
    let file_name = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_size_bytes = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(0);

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
pub fn render_page_thumbnail(engine: State<'_, PdfEngine>, path: String, page_index: usize, width_px: Option<u32>) -> Result<String, String> {
    use image::RgbaImage;
    let doc = engine.open_document(&path)?;
    let idx: i32 = page_index.try_into().map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc.pages().get(idx).map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let width: i32 = width_px.unwrap_or(120) as i32;
    let config = pdfium_render::prelude::PdfRenderConfig::new().set_target_width(width);
    let bitmap = page.render_with_config(&config).map_err(|e| format!("Render error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("frappe_pdf");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Temp dir error: {}", e))?;
    let out_path = temp_dir.join(format!("thumb_{page_index}.png"));
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]));
        }
    }
    img.save(&out_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(out_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn render_page(engine: State<'_, PdfEngine>, path: String, page_index: usize, dpi: Option<f32>) -> Result<String, String> {
    use image::RgbaImage;
    use pdfium_render::prelude::PdfRenderConfig;
    let doc = engine.open_document(&path)?;
    let idx: i32 = page_index.try_into().map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc.pages().get(idx).map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let dpi_val = dpi.unwrap_or(144.0) as f64;
    let page_width = page.width().value as f64;
    let px_width = (page_width * dpi_val / 72.0) as i32;
    let config = PdfRenderConfig::new().set_target_width(px_width);
    let bitmap = page.render_with_config(&config).map_err(|e| format!("Render error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("frappe_pdf");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Temp dir error: {}", e))?;
    let out_path = temp_dir.join(format!("page_{page_index}.png"));
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]));
        }
    }
    img.save(&out_path).map_err(|e| format!("Save error: {}", e))?;
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
pub fn render_page_with_overprint(engine: State<'_, PdfEngine>, path: String, page_index: usize, dpi: Option<f32>) -> Result<String, String> {
    use image::RgbaImage;
    use pdfium_render::prelude::PdfRenderConfig;
    let doc = engine.open_document(&path)?;
    let idx: i32 = page_index.try_into().map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc.pages().get(idx).map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let dpi_val = dpi.unwrap_or(144.0) as f64;
    let page_width = page.width().value as f64;
    let px_width = (page_width * dpi_val / 72.0) as i32;
    let config = PdfRenderConfig::new().set_target_width(px_width).use_print_quality(true);
    let bitmap = page.render_with_config(&config).map_err(|e| format!("Render error: {}", e))?;
    let temp_dir = std::env::temp_dir().join("frappe_pdf");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Temp dir error: {}", e))?;
    let out_path = temp_dir.join(format!("page_{page_index}_overprint.png"));
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(x, y, image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]));
        }
    }
    img.save(&out_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(out_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_page_dimensions(engine: State<'_, PdfEngine>, path: String, page_index: usize) -> Result<PageDimensions, String> {
    let doc = engine.open_document(&path)?;
    let idx: i32 = page_index.try_into().map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc.pages().get(idx).map_err(|e| format!("Page {page_index} not found: {e}"))?;
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
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let pages = doc.get_pages();
    let all_page_numbers: Vec<u32> = pages.keys().copied().collect();
    let to_keep: std::collections::HashSet<u32> = indices.iter().filter_map(|i| all_page_numbers.get(*i)).copied().collect();
    let to_remove: Vec<u32> = all_page_numbers.iter().filter(|pn| !to_keep.contains(pn)).copied().collect();
    doc.delete_pages(&to_remove);
    doc.save(&output_path).map_err(|e| format!("Failed to save: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn delete_pages(path: String, indices: Vec<usize>, output_path: String) -> Result<(), String> {
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let pages = doc.get_pages();
    let all_page_numbers: Vec<u32> = pages.keys().copied().collect();
    let to_remove: Vec<u32> = indices.iter().filter_map(|i| all_page_numbers.get(*i)).copied().collect();
    doc.delete_pages(&to_remove);
    doc.save(&output_path).map_err(|e| format!("Failed to save: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn rotate_page(path: String, page_index: usize, degrees: i64, output_path: String) -> Result<(), String> {
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let pages = doc.get_pages();
    let obj_id = match pages.get(&(page_index as u32)) {
        Some(id) => *id,
        None => return Err(format!("Page {} not found", page_index)),
    };
    if let Ok(page) = doc.get_dictionary_mut(obj_id) {
        page.set("Rotate", Object::Integer(degrees));
    }
    doc.save(&output_path).map_err(|e| format!("Failed to save: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn check_fonts(path: String) -> Result<Vec<FontFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::fonts::collect_fonts(&doc))
}

#[tauri::command]
pub fn check_page_boxes(path: String) -> Result<Vec<PageBoxFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::boxes::check_page_boxes(&doc))
}

#[tauri::command]
pub fn check_image_resolution(path: String) -> Result<Vec<ImageResolutionFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::images::check_image_resolution(&doc))
}

#[tauri::command]
pub fn check_bleed(path: String, min_bleed_mm: Option<f64>) -> Result<Vec<BleedFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let min = min_bleed_mm.unwrap_or(3.0);
    Ok(crate::pdf::bleed::check_bleed(&doc, min))
}

#[tauri::command]
pub fn add_bleed(path: String, amount_mm: f64, output_path: String) -> Result<(), String> {
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
                if vals.len() == 4 { Some(vals) } else { None }
            } else {
                None
            }
        })
    }

    for obj_id in &page_ids {
        let page_dict = doc.get_dictionary_mut(*obj_id)
            .map_err(|e| format!("Failed to get page dict: {}", e))?;

        let bleed_vals = get_array_vals(page_dict, b"BleedBox");
        let new_bleed = if let Some(bb) = bleed_vals {
            vec![bb[0] - amount_pts, bb[1] - amount_pts, bb[2] + amount_pts, bb[3] + amount_pts]
        } else if let Some(trim) = get_array_vals(page_dict, b"TrimBox") {
            vec![trim[0] - amount_pts, trim[1] - amount_pts, trim[2] + amount_pts, trim[3] + amount_pts]
        } else {
            continue;
        };

        page_dict.set("BleedBox", lopdf::Object::Array(vec![
            lopdf::Object::Real(new_bleed[0] as f32),
            lopdf::Object::Real(new_bleed[1] as f32),
            lopdf::Object::Real(new_bleed[2] as f32),
            lopdf::Object::Real(new_bleed[3] as f32),
        ]));

        // Expand MediaBox if needed
        if let Some(media) = get_array_vals(page_dict, b"MediaBox") {
            let new_media = vec![
                media[0].min(new_bleed[0]),
                media[1].min(new_bleed[1]),
                media[2].max(new_bleed[2]),
                media[3].max(new_bleed[3]),
            ];
            if new_media != media {
                page_dict.set("MediaBox", lopdf::Object::Array(vec![
                    lopdf::Object::Real(new_media[0] as f32),
                    lopdf::Object::Real(new_media[1] as f32),
                    lopdf::Object::Real(new_media[2] as f32),
                    lopdf::Object::Real(new_media[3] as f32),
                ]));
            }
        }
    }

    doc.save(&output_path).map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn check_output_intents(path: String) -> Result<Vec<OutputIntent>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::metadata::get_output_intents(&doc))
}

#[tauri::command]
pub fn check_security(path: String) -> Result<Vec<SecurityFinding>, String> {
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
pub fn check_color_spaces(path: String, target_profile: String) -> Result<Vec<ColorSpaceFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_color_spaces(&doc, &target_profile))
}

#[tauri::command]
pub fn check_overprint(path: String) -> Result<Vec<OverprintFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_overprint(&doc))
}

#[tauri::command]
pub fn check_transparency(path: String) -> Result<Vec<TransparencyFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_transparency(&doc))
}

#[tauri::command]
pub fn check_hidden_content(path: String) -> Result<Vec<HiddenContentFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_hidden_content(&doc))
}

#[tauri::command]
pub fn check_spot_colors(path: String) -> Result<Vec<SpotColorFinding>, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_spot_colors(&doc))
}

#[tauri::command]
pub fn check_ink_coverage() -> Vec<InkCoverageFinding> {
    crate::pdf::color::check_ink_coverage()
}

#[tauri::command]
pub fn list_icc_profiles() -> Vec<IccProfileInfo> {
    crate::pdf::transforms::get_bundled_icc_profiles()
}

#[tauri::command]
pub fn convert_rgb_to_cmyk(path: String, output_path: String) -> Result<(), String> {
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    crate::pdf::transforms::convert_rgb_to_cmyk(&mut doc)?;
    doc.save(&output_path).map_err(|e| format!("Failed to save converted PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn add_output_intent(path: String, output_path: String, icc_profile: String, condition_id: String, condition: String) -> Result<(), String> {
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    // The ICC profile file path is passed; read it
    let icc_data = std::fs::read(&icc_profile).map_err(|e| format!("Failed to read ICC profile: {}", e))?;
    crate::pdf::transforms::add_output_intent(&mut doc, &icc_data, &condition_id, &condition)?;
    doc.save(&output_path).map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_pdf_catalog(path: String) -> Result<serde_json::Value, String> {
    let doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let catalog = doc.get_object((1, 0)).map_err(|e| format!("Failed to get catalog: {}", e))?;
    let dict = catalog.as_dict().map_err(|_| "Catalog is not a dictionary".to_string())?;

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
    result.insert("PageCount".to_string(), serde_json::Value::Number(serde_json::Number::from(page_count as u64)));

    // Add PDF version
    let pdf_version = {
        let path_buf = std::path::PathBuf::from(&path);
        let mut header = [0u8; 100];
        if let Ok(file) = std::fs::File::open(&path_buf) {
            use std::io::Read;
            let mut reader = std::io::BufReader::new(file);
            if reader.read(&mut header).is_ok() {
                String::from_utf8_lossy(&header).lines().next()
                    .and_then(|l| if l.trim().starts_with("%PDF-") { Some(l.trim()[5..].to_string()) } else { None })
                    .unwrap_or_else(|| "unknown".to_string())
            } else { "unknown".to_string() }
        } else { "unknown".to_string() }
    };
    result.insert("PDFVersion".to_string(), serde_json::Value::String(pdf_version));

    Ok(serde_json::Value::Object(result))
}

// ── Preflight findings persistence (Days 43-44) ────────────────────────────

#[tauri::command]
pub fn save_preflight_run(db: State<'_, Database>, job_id: i64, profile: String, findings: Vec<PreflightFindingInput>) -> Result<i64, String> {
    db.save_preflight_run(job_id, &profile, &findings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_preflight_runs(db: State<'_, Database>, job_id: i64) -> Result<Vec<PreflightRunSummary>, String> {
    db.list_preflight_runs(job_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_findings_for_run(db: State<'_, Database>, run_id: i64) -> Result<Vec<PreflightFinding>, String> {
    db.list_findings_for_run(run_id).map_err(|e| e.to_string())
}
