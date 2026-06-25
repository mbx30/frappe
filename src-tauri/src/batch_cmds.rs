//! Batched read-only command dispatch (Issue #291).
//!
//! Allows multiple small read-only Tauri commands to be executed in a
//! single IPC round-trip, reducing latency for dashboard-style views.

use tauri::State;

use crate::db::Database;

/// A single batched command.
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

/// Run a list of small Tauri commands in a single IPC round-trip.
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
