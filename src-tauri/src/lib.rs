mod cloud_import;
mod commands;
mod db;
mod import;
mod models;
mod pdf;

use crate::pdf::engine::PdfEngine;
use db::Database;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle();
            let app_dir: PathBuf = app_handle.path().app_data_dir().expect("failed to get app data dir");
            let database = Database::new(app_dir.clone()).expect("failed to initialize database");

            let pdf_engine = PdfEngine::init().expect("failed to initialize PDF engine");
            app_handle.manage(pdf_engine);

            // Verify database integrity on startup
            let verification_result = database.verify_integrity();
            if !verification_result.is_valid {
                eprintln!("Database verification failed:");
                for error in &verification_result.errors {
                    eprintln!("  ERROR: {}", error);
                }
            }
            if !verification_result.warnings.is_empty() {
                for warning in &verification_result.warnings {
                    eprintln!("  WARNING: {}", warning);
                }
            }

            app_handle.manage(database);
            if cfg!(debug_assertions) {
                app_handle.plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_workbook,
            commands::list_workbooks,
            commands::delete_workbook,
            commands::get_workbook,
            commands::create_sheet,
            commands::add_column,
            commands::update_cell_value,
            commands::add_row,
            commands::update_workbook_name,
            commands::import_csv_file,
            commands::import_excel_file,
            commands::import_google_sheet,
            commands::import_notion_database,
            commands::preview_import,
            commands::verify_database,
            commands::get_business_info,
            commands::save_business_info,
            commands::create_invoice,
            commands::list_invoices,
            commands::get_invoice,
            commands::add_invoice_line_item,
            commands::replace_invoice_line_items,
            commands::update_invoice,
            commands::create_order,
            commands::list_orders,
            commands::get_order,
            commands::update_order_status,
            commands::update_order,
            commands::create_estimate,
            commands::list_estimates,
            commands::get_estimate,
            commands::add_estimate_line_item,
            commands::replace_estimate_line_items,
            commands::update_estimate,
            commands::add_inventory_item,
            commands::list_inventory_items,
            commands::get_inventory_item,
            commands::adjust_inventory,
            commands::get_low_stock_alerts,
            commands::acknowledge_alert,
            commands::create_client,
            commands::list_clients,
            commands::get_client,
            commands::update_client,
            commands::delete_client,
            commands::create_art_approval,
            commands::get_art_approvals_for_order,
            commands::respond_to_art_approval,
            commands::increment_art_approval_follow_up,
            commands::record_payment,
            commands::list_payments,
            commands::delete_payment,
            commands::search_invoices_and_orders,
            commands::log_invoice_reminder,
            commands::list_invoice_reminders,
            commands::update_invoice_qb_status,
            commands::update_order_job_specs,
            commands::update_order_fulfillment,
            commands::add_department_note,
            commands::list_department_notes,
            commands::delete_department_note,
            commands::open_pdf,
            commands::save_pdf_job,
            commands::list_pdf_jobs,
            commands::delete_pdf_job,
            commands::render_page_thumbnail,
            commands::render_page,
            commands::check_fonts,
            commands::check_page_boxes,
            commands::check_image_resolution,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
