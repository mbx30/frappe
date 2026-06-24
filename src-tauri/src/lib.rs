mod cache;
mod cloud_backup;
mod cloud_import;
mod commands;
mod db;
mod email;
mod ftp;
mod import;
mod keychain;
mod logging;
pub mod metrics;
mod models;
mod ai_check;
mod observability;
pub mod pdf;
pub mod security;


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

            // PDF engine is optional — invoice/estimate/order/inventory features
            // work without it. Only thumbnail/preflight/page-render features
            // require it. If init fails, log the error and continue so the
            // user can still use 95% of the app.
            let pdf_engine = PdfEngine::init();
            app_handle.manage(pdf_engine);

            // Verify database integrity on startup
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

            // Issue #269 — auto-start any active hot folders BEFORE we
            // move the database into managed state, so the watcher setup
            // can read the folder list without an extra borrow.
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

            // Issue #256 — record cold-start time once the runtime is ready.
            metrics::record_cold_start();

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
            commands::next_order_number,
            commands::create_invoice,
            commands::list_invoices,
            commands::list_invoices_paginated,
            commands::get_invoice,
            commands::add_invoice_line_item,
            commands::replace_invoice_line_items,
            commands::update_invoice,
            commands::create_order,
            commands::list_orders,
            commands::list_orders_paginated,
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
            commands::list_clients_paginated,
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
            commands::check_bleed,
            commands::add_bleed,
            commands::check_output_intents,
            commands::check_security,
            commands::check_full_preflight,
            commands::check_pdfx,
            commands::check_color_spaces,
            commands::check_overprint,
            commands::check_transparency,
            commands::check_hidden_content,
            commands::check_spot_colors,
            commands::check_ink_coverage,
            commands::list_icc_profiles,
            commands::convert_rgb_to_cmyk,
            commands::add_output_intent,
            commands::get_pdf_catalog,
            commands::render_page_with_overprint,
            commands::get_page_dimensions,
            commands::extract_pages,
            commands::delete_pages,
            commands::rotate_page,
            commands::save_preflight_run,
            commands::list_preflight_runs,
            commands::list_findings_for_run,
            commands::create_certified_version,
            commands::list_certified_versions,
            // Phase 3.2
            commands::reorder_pages,
            commands::insert_blank_page,
            commands::list_layers,
            commands::set_layer_visibility,
            // Phase 3.3
            commands::decode_content_stream,
            commands::encode_content_stream,
            commands::round_trip_page,
            commands::tokenize_content_stream,
            // Phase 3.4
            commands::search_text,
            commands::replace_text,
            // Phase 3.5
            commands::replace_image,
            commands::optimize_image,
            // Phase 4.1
            commands::generate_approval_sheet,
            commands::export_preflight_report_json,
            commands::export_preflight_report_csv,
            commands::get_check_registry,
            commands::run_profile,
            commands::create_preflight_profile,
            commands::list_preflight_profiles,
            commands::get_preflight_profile,
            commands::delete_preflight_profile,
            commands::list_profile_checks,
            commands::update_profile_check,
            commands::list_profile_fixups,
            commands::update_profile_fixup,
            // Phase 4.2
            commands::create_action_list,
            commands::list_action_lists,
            commands::get_action_list,
            commands::delete_action_list,
            commands::add_action_list_step,
            commands::list_action_list_steps,
            commands::delete_action_list_step,
            commands::reorder_action_list_steps,
            commands::start_action_recording,
            commands::record_action_step,
            commands::stop_action_recording,
            commands::cancel_action_recording,
            commands::is_action_recording,
            commands::replay_action_list,
            commands::create_debug_session,
            commands::list_debug_sessions,
            commands::get_debug_session,
            commands::delete_debug_session,
            commands::step_forward_debug,
            commands::run_from_here_debug,
            commands::render_debug_thumbnail,
            commands::export_debug_report_pdf,
            // Phase 4.3
            commands::create_batch_job,
            commands::list_batch_jobs,
            commands::get_batch_job,
            commands::run_batch,
            commands::list_batch_results,
            // Phase 4.5
            commands::create_hot_folder,
            commands::list_hot_folders,
            commands::delete_hot_folder,
            commands::toggle_hot_folder,
            commands::start_hot_folder_watcher,
            commands::stop_hot_folder_watcher,
            // Phase 5.1
            commands::compress_pdf,
            // Phase 6.1 — Redaction (#231)
            commands::redact_pdf,
            commands::get_redaction_audit_log,
            commands::verify_redaction_chain,
            // Phase 5.2
            commands::detect_barcodes,
            // Phase 5.3
            commands::get_analytics_summary,
            commands::get_analytics_dashboard,
            // Phase 5.5
            commands::ai_visual_check,
            // Phase 6.1
            commands::save_email_settings,
            commands::get_email_settings,
            commands::send_email,
            commands::save_ftp_settings,
            commands::get_ftp_settings,
            commands::ftp_upload,
            commands::create_webhook,
            commands::list_webhooks,
            commands::delete_webhook,
            // #84 — Job ticket
            commands::generate_job_ticket,
            // #85 — Cloud backup
            commands::upload_event_batch_cmd,
            commands::upload_snapshot_cmd,
            commands::get_cloud_backup_status,
            // #89 — Keychain
            commands::keychain_read,
            commands::keychain_write,
            commands::keychain_delete,
            // #90 — DB operations
            commands::get_schema_version,
            commands::create_backup,
            commands::list_backups,
            // #99 — SQLCipher encryption
            commands::export_plaintext_backup,
            // #88 — Observability
            commands::reveal_logs,
            commands::crash_report,
            // Issue #256 — metrics snapshot for the PerfOverlay.
            commands::get_metrics_snapshot,
            // Issue #241 / #275 — Preferences + PDF settings
            commands::get_preference,
            commands::set_preference,
            commands::get_all_preferences,
            // Issue #234 — Alt text editor
            commands::get_alt_text,
            commands::list_alt_text,
            commands::set_alt_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
