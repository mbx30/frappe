mod ai_check;
mod ai_cmds;
mod analytics_cmds;
mod batch_cmds;
mod cache;
mod cloud_backup;
mod cloud_import;
mod comm_cmds;
mod commands;
pub mod commands_extra;
mod db;
mod db_cmds;
mod email;
mod ftp;
mod import_cmds;
mod import;
mod job_cmds;
mod keychain;
mod logging;
pub mod metrics;
mod models;
pub mod pdf;
pub mod pdf_cmds;
mod preflight_cmds;
pub mod security;
mod settings_cmds;
mod text_cmds;
mod workbook_cmds;
mod observability;

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
            let app_dir: PathBuf = app_handle
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");

            let logging_guard = logging::init_logging(&app_dir);
            app_handle.manage(logging_guard);
            tracing::info!("Frappe starting up");

            let database = Database::new(app_dir.clone()).expect("failed to initialize database");

            let pdf_engine = PdfEngine::init();
            app_handle.manage(pdf_engine);

            let verification_result = database.verify_integrity();
            if !verification_result.is_valid {
                tracing::error!("Database verification failed");
                for error in &verification_result.errors {
                    tracing::error!("  ERROR: {}", error);
                }
            }
            if !verification_result.warnings.is_empty() {
                for warning in &verification_result.warnings {
                    tracing::warn!("  WARNING: {}", warning);
                }
            }

            {
                let ah = app.handle().clone();
                if let Ok(folders) = database.list_hot_folders() {
                    for folder in folders {
                        if !folder.is_active {
                            continue;
                        }
                        let cfg = crate::pdf::watcher::HotFolderConfig {
                            watch_path: folder.watch_path.clone(),
                            action_list_id: folder.action_list_id,
                            output_path: folder.output_path.clone(),
                            file_pattern: folder.file_pattern.clone(),
                            max_concurrency: None,
                            max_queue_depth: None,
                            max_write_retries: None,
                            stability_poll_ms: None,
                        };
                        if let Err(e) =
                            crate::pdf::watcher::start_hot_folder_watcher(cfg, Some(ah.clone()))
                        {
                            tracing::warn!(
                                "hot folder '{}' failed to start: {}",
                                folder.name,
                                e
                            );
                        } else {
                            tracing::info!("hot folder '{}' watcher started", folder.name);
                        }
                    }
                }
            }

            app_handle.manage(database);

            metrics::record_cold_start();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Workbook
            workbook_cmds::create_workbook,
            workbook_cmds::list_workbooks,
            workbook_cmds::delete_workbook,
            workbook_cmds::get_workbook,
            workbook_cmds::create_sheet,
            workbook_cmds::add_column,
            workbook_cmds::update_cell_value,
            workbook_cmds::add_row,
            workbook_cmds::update_workbook_name,
            // Import
            import_cmds::import_csv_file,
            import_cmds::import_excel_file,
            import_cmds::import_google_sheet,
            import_cmds::import_notion_database,
            import_cmds::preview_import,
            // Database admin
            db_cmds::verify_database,
            db_cmds::get_schema_version,
            db_cmds::create_backup,
            db_cmds::list_backups,
            db_cmds::export_plaintext_backup,
            // Business
            commands::get_business_info,
            commands::save_business_info,
            commands::next_order_number,
            // Invoices
            commands::create_invoice,
            commands::list_invoices,
            commands::list_invoices_paginated,
            commands::get_invoice,
            commands::add_invoice_line_item,
            commands::replace_invoice_line_items,
            commands::update_invoice,
            // Orders
            commands::create_order,
            commands::list_orders,
            commands::list_orders_paginated,
            commands::get_order,
            commands::update_order_status,
            commands::update_order,
            // Estimates
            commands::create_estimate,
            commands::list_estimates,
            commands::get_estimate,
            commands::add_estimate_line_item,
            commands::replace_estimate_line_items,
            commands::update_estimate,
            // Inventory
            commands::add_inventory_item,
            commands::list_inventory_items,
            commands::get_inventory_item,
            commands::adjust_inventory,
            commands::get_low_stock_alerts,
            commands::acknowledge_alert,
            // Clients
            commands::create_client,
            commands::list_clients,
            commands::list_clients_paginated,
            commands::get_client,
            commands::update_client,
            commands::delete_client,
            // Art approvals
            commands::create_art_approval,
            commands::get_art_approvals_for_order,
            commands::respond_to_art_approval,
            commands::increment_art_approval_follow_up,
            // Payments
            commands::record_payment,
            commands::list_payments,
            commands::delete_payment,
            // Search
            commands::search_invoices_and_orders,
            // Invoice reminders
            commands::log_invoice_reminder,
            commands::list_invoice_reminders,
            // QuickBooks
            commands::update_invoice_qb_status,
            // Job specs & fulfillment
            commands::update_order_job_specs,
            commands::update_order_fulfillment,
            // Department notes
            commands::add_department_note,
            commands::list_department_notes,
            commands::delete_department_note,
            // PDF open/save/certified
            pdf_cmds::open_pdf,
            pdf_cmds::save_pdf_job,
            pdf_cmds::list_pdf_jobs,
            pdf_cmds::delete_pdf_job,
            pdf_cmds::create_certified_version,
            pdf_cmds::list_certified_versions,
            // PDF rendering
            pdf_cmds::render_page_thumbnail,
            pdf_cmds::render_page,
            pdf_cmds::render_page_with_overprint,
            pdf_cmds::get_page_dimensions,
            // Page operations
            pdf_cmds::extract_pages,
            pdf_cmds::delete_pages,
            pdf_cmds::rotate_page,
            pdf_cmds::reorder_pages,
            pdf_cmds::insert_blank_page,
            pdf_cmds::get_pdf_catalog,
            // Layers
            pdf_cmds::list_layers,
            pdf_cmds::set_layer_visibility,
            // Content stream
            pdf_cmds::decode_content_stream,
            pdf_cmds::encode_content_stream,
            pdf_cmds::round_trip_page,
            pdf_cmds::tokenize_content_stream,
            // Text
            text_cmds::search_text,
            text_cmds::replace_text,
            // Image ops
            pdf_cmds::replace_image,
            pdf_cmds::optimize_image,
            // Preflight checks
            preflight_cmds::check_fonts,
            preflight_cmds::check_page_boxes,
            preflight_cmds::check_image_resolution,
            preflight_cmds::check_bleed,
            preflight_cmds::add_bleed,
            preflight_cmds::check_output_intents,
            preflight_cmds::check_security,
            preflight_cmds::check_full_preflight,
            preflight_cmds::check_pdfx,
            preflight_cmds::check_color_spaces,
            preflight_cmds::check_overprint,
            preflight_cmds::check_transparency,
            preflight_cmds::check_hidden_content,
            preflight_cmds::check_spot_colors,
            preflight_cmds::check_ink_coverage,
            preflight_cmds::list_icc_profiles,
            preflight_cmds::convert_rgb_to_cmyk,
            preflight_cmds::add_output_intent,
            // Preflight profiles
            preflight_cmds::save_preflight_run,
            preflight_cmds::list_preflight_runs,
            preflight_cmds::list_findings_for_run,
            preflight_cmds::get_check_registry,
            preflight_cmds::run_profile,
            preflight_cmds::create_preflight_profile,
            preflight_cmds::list_preflight_profiles,
            preflight_cmds::get_preflight_profile,
            preflight_cmds::delete_preflight_profile,
            preflight_cmds::list_profile_checks,
            preflight_cmds::update_profile_check,
            preflight_cmds::list_profile_fixups,
            preflight_cmds::update_profile_fixup,
            // Action lists
            preflight_cmds::create_action_list,
            preflight_cmds::list_action_lists,
            preflight_cmds::get_action_list,
            preflight_cmds::delete_action_list,
            preflight_cmds::add_action_list_step,
            preflight_cmds::list_action_list_steps,
            preflight_cmds::delete_action_list_step,
            preflight_cmds::reorder_action_list_steps,
            preflight_cmds::start_action_recording,
            preflight_cmds::record_action_step,
            preflight_cmds::stop_action_recording,
            preflight_cmds::cancel_action_recording,
            preflight_cmds::is_action_recording,
            preflight_cmds::replay_action_list,
            // Debug sessions
            preflight_cmds::create_debug_session,
            preflight_cmds::list_debug_sessions,
            preflight_cmds::get_debug_session,
            preflight_cmds::delete_debug_session,
            preflight_cmds::step_forward_debug,
            preflight_cmds::run_from_here_debug,
            preflight_cmds::render_debug_thumbnail,
            preflight_cmds::export_debug_report_pdf,
            // Batch processing
            preflight_cmds::create_batch_job,
            preflight_cmds::list_batch_jobs,
            preflight_cmds::get_batch_job,
            preflight_cmds::run_batch,
            preflight_cmds::list_batch_results,
            // Hot folders
            preflight_cmds::create_hot_folder,
            preflight_cmds::list_hot_folders,
            preflight_cmds::delete_hot_folder,
            preflight_cmds::toggle_hot_folder,
            preflight_cmds::start_hot_folder_watcher,
            preflight_cmds::stop_hot_folder_watcher,
            // Export
            preflight_cmds::generate_approval_sheet,
            preflight_cmds::export_preflight_report_json,
            preflight_cmds::export_preflight_report_csv,
            // PDF compression
            preflight_cmds::compress_pdf,
            // Redaction
            preflight_cmds::redact_pdf,
            preflight_cmds::get_redaction_audit_log,
            preflight_cmds::verify_redaction_chain,
            // Barcode
            preflight_cmds::detect_barcodes,
            // Analytics
            analytics_cmds::get_analytics_summary,
            analytics_cmds::get_analytics_dashboard,
            // AI / OCR
            ai_cmds::ai_visual_check,
            // Communication
            comm_cmds::save_email_settings,
            comm_cmds::get_email_settings,
            comm_cmds::send_email,
            comm_cmds::save_ftp_settings,
            comm_cmds::get_ftp_settings,
            comm_cmds::ftp_upload,
            comm_cmds::create_webhook,
            comm_cmds::list_webhooks,
            comm_cmds::delete_webhook,
            // Job ticket
            job_cmds::generate_job_ticket,
            // Cloud backup
            commands::upload_event_batch_cmd,
            commands::upload_snapshot_cmd,
            commands::get_cloud_backup_status,
            // Keychain
            commands::keychain_read,
            commands::keychain_write,
            commands::keychain_delete,
            // Observability
            commands::reveal_logs,
            commands::crash_report,
            commands::get_metrics_snapshot,
            // Preferences & alt text
            settings_cmds::get_preference,
            settings_cmds::set_preference,
            settings_cmds::get_all_preferences,
            settings_cmds::get_alt_text,
            settings_cmds::list_alt_text,
            settings_cmds::set_alt_text,
            // PDF annotations
            pdf_cmds::pdf_annotation_add,
            pdf_cmds::pdf_annotations_list,
            pdf_cmds::pdf_annotation_update,
            pdf_cmds::pdf_annotation_delete,
            pdf_cmds::pdf_annotation_page_counts,
            pdf_cmds::pdf_annotation_reply_add,
            pdf_cmds::pdf_annotation_replies_list,
            // Events and rendering
            commands_extra::subscribe_events,
            commands_extra::render_page_b64,
            // Batched IPC
            batch_cmds::batch_commands,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
