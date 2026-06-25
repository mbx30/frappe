//! Main command dispatcher.
//!
//! Domain-specific commands have been extracted into separate modules:
//!   - workbook_cmds  — workbook/sheet/cell CRUD
//!   - import_cmds    — CSV/Excel/Google/Notion import
//!   - db_cmds        — database admin (verify, backup, schema)
//!   - preflight_cmds — PDF preflight checks, profiles, action lists, redaction, batch
//!   - pdf_cmds       — PDF open/save/render/page-ops/images/annotations/compression
//!   - text_cmds      — search/replace text in PDFs
//!   - analytics_cmds — analytics summary/dashboard
//!   - ai_cmds        — AI visual check, OCR
//!   - comm_cmds      — email, FTP, webhook
//!   - settings_cmds  — preferences, alt-text
//!   - job_cmds       — job ticket generation
//!   - batch_cmds     — batched read-only IPC
//!
//! This file retains the business CRUD commands (invoices, orders, estimates,
//! clients, inventory, payments, art approvals, etc.) plus system commands
//! (cloud backup, keychain, observability, metrics).

use tauri::State;

use crate::db::Database;
use crate::models::*;
use crate::security;

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

// ── Invoices ─────────────────────────────────────────────────────────────

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

// ── Orders ───────────────────────────────────────────────────────────────

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

// ── Estimates ────────────────────────────────────────────────────────────

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

// ── Inventory ────────────────────────────────────────────────────────────

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

// ── Clients ──────────────────────────────────────────────────────────────

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

// ── Art approvals ────────────────────────────────────────────────────────

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

// ── Payments ─────────────────────────────────────────────────────────────

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

// ── Search ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn search_invoices_and_orders(
    db: State<'_, Database>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    db.search_invoices_and_orders(&query)
        .map_err(|e| e.to_string())
}

// ── Invoice reminders ────────────────────────────────────────────────────

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

// ── QuickBooks sync ──────────────────────────────────────────────────────

#[tauri::command]
pub fn update_invoice_qb_status(
    db: State<'_, Database>,
    id: i64,
    status: String,
) -> Result<(), String> {
    db.update_invoice_qb_status(id, &status)
        .map_err(|e| e.to_string())
}

// ── Job specs & fulfillment ─────────────────────────────────────────────

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

// ── Department notes ─────────────────────────────────────────────────────

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

// ── Cloud backup (#85) ──────────────────────────────────────────────────

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
    let validated_path = security::validate_read_path(&file_path)?;
    let checksum = compute_snapshot_checksum(&validated_path.to_string_lossy());
    let snapshot = crate::cloud_backup::SnapshotUpload {
        tenant_id,
        file_path,
        checksum,
    };
    let result = crate::cloud_backup::upload_snapshot(&snapshot).await?;
    Ok(result.message)
}

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

// ── Keychain (#89) ──────────────────────────────────────────────────────

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

// ── Observability (#88, #256) ───────────────────────────────────────────

#[tauri::command]
pub async fn crash_report(
    error_message: String,
    stack_trace: String,
) -> Result<crate::observability::CrashResponse, String> {
    crate::observability::crash_report(error_message, stack_trace).await
}

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

#[tauri::command]
pub fn get_metrics_snapshot() -> crate::metrics::MetricsSnapshot {
    crate::metrics::snapshot()
}
