mod cloud_import;
mod commands;
mod db;
mod import;
mod models;

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
            let database = Database::new(app_dir).expect("failed to initialize database");

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
            commands::update_estimate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
