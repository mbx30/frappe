//! Analytics dashboard commands (Phase 5.3).
//!
//! Summary and dashboard queries for preflight analytics, client pass rates,
//! turnaround times, and error categories.

use tauri::State;

use crate::db::Database;
use crate::models::{AnalyticsDashboard, AnalyticsSummary};

#[tauri::command]
pub fn get_analytics_summary(db: State<'_, Database>) -> Result<AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

/// Combined analytics payload for the dashboard: per-client pass
/// rates, average order turnaround (hours), and the most common
/// error categories.
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
