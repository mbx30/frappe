//! Action list debugger (#268).
//!
//! Persists debug sessions in a new SQLite table
//! `action_list_debug_sessions` so an operator can step forward, run
//! from a given step, and re-open the same debug later. Each session
//! records the original PDF, the action list, the current step index,
//! and a serialized `Vec<StepResult>` from the replay engine.
//!
//! A debug report PDF can be exported with `printpdf` that summarises
//! each step, the original and final byte sizes, and the list of
//! `before` / `after` PDF paths produced by the replay.

use crate::db::Database;
use crate::pdf::action_list::{ActionStep, ReplayResult, StepResult};
use lopdf::Document;
use printpdf::{BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerReference, Rgb};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    pub id: Option<i64>,
    pub name: String,
    pub pdf_path: String,
    pub steps: Vec<ActionStep>,
    pub step_index: i64,
    pub step_results: Vec<StepResult>,
    pub final_output: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn ensure_debug_table(db: &Database) -> Result<(), String> {
    let conn = db
        .conn
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS action_list_debug_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            pdf_path TEXT NOT NULL,
            steps_json TEXT NOT NULL,
            step_index INTEGER NOT NULL DEFAULT 0,
            step_results_json TEXT NOT NULL DEFAULT '[]',
            final_output TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create_debug_session(
    db: &Database,
    name: &str,
    pdf_path: &str,
    steps: &[ActionStep],
) -> Result<DebugSession, String> {
    ensure_debug_table(db)?;
    let conn = db
        .conn
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .map_err(|e| e.to_string())?;
    let steps_json = serde_json::to_string(steps).map_err(|e| e.to_string())?;
    let results_json = "[]".to_string();
    conn.execute(
        "INSERT INTO action_list_debug_sessions (name, pdf_path, steps_json, step_index, step_results_json) VALUES (?1, ?2, ?3, 0, ?4)",
        params![name, pdf_path, steps_json, results_json],
    ).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    drop(conn);
    Ok(DebugSession {
        id: Some(id),
        name: name.to_string(),
        pdf_path: pdf_path.to_string(),
        steps: steps.to_vec(),
        step_index: 0,
        step_results: Vec::new(),
        final_output: None,
        created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

pub fn update_debug_session(
    db: &Database,
    session: &DebugSession,
) -> Result<(), String> {
    let id = session.id.ok_or_else(|| "session has no id".to_string())?;
    let conn = db
        .conn
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .map_err(|e| e.to_string())?;
    let steps_json = serde_json::to_string(&session.steps).map_err(|e| e.to_string())?;
    let results_json = serde_json::to_string(&session.step_results).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE action_list_debug_sessions SET steps_json = ?1, step_index = ?2, step_results_json = ?3, final_output = ?4, updated_at = datetime('now') WHERE id = ?5",
        params![steps_json, session.step_index, results_json, session.final_output, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_debug_sessions(db: &Database) -> Result<Vec<DebugSession>, String> {
    ensure_debug_table(db)?;
    let conn = db
        .conn
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, pdf_path, steps_json, step_index, step_results_json, final_output, created_at, updated_at FROM action_list_debug_sessions ORDER BY updated_at DESC LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let steps_json: String = row.get(3)?;
            let results_json: String = row.get(5)?;
            Ok(DebugSession {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                pdf_path: row.get(2)?,
                steps: serde_json::from_str(&steps_json).unwrap_or_default(),
                step_index: row.get(4)?,
                step_results: serde_json::from_str(&results_json).unwrap_or_default(),
                final_output: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_debug_session(db: &Database, id: i64) -> Result<DebugSession, String> {
    ensure_debug_table(db)?;
    let conn = db
        .conn
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, pdf_path, steps_json, step_index, step_results_json, final_output, created_at, updated_at FROM action_list_debug_sessions WHERE id = ?1",
        params![id],
        |row| {
            let steps_json: String = row.get(3)?;
            let results_json: String = row.get(5)?;
            Ok(DebugSession {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                pdf_path: row.get(2)?,
                steps: serde_json::from_str(&steps_json).unwrap_or_default(),
                step_index: row.get(4)?,
                step_results: serde_json::from_str(&results_json).unwrap_or_default(),
                final_output: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn delete_debug_session(db: &Database, id: i64) -> Result<(), String> {
    let conn = db
        .conn
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM action_list_debug_sessions WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Execute a single step and update the session in place. Returns the
/// updated session. Persists the new step_index and step_results.
pub fn step_forward(
    db: &Database,
    session_id: i64,
    working_dir: &Path,
) -> Result<DebugSession, String> {
    let mut session = get_debug_session(db, session_id)?;
    if (session.step_index as usize) >= session.steps.len() {
        return Err("Already at the end of the list".to_string());
    }
    let step_index = session.step_index as usize;
    let step = session.steps[step_index].clone();
    let input_pdf: std::path::PathBuf = if let Some(prev) = session
        .step_results
        .last()
        .and_then(|r| r.output_path.clone())
    {
        std::path::PathBuf::from(prev)
    } else {
        std::path::PathBuf::from(&session.pdf_path)
    };
    let r = crate::pdf::action_list::replay(&input_pdf, &[step.clone()], working_dir)?;
    let r = r.steps.into_iter().next().unwrap_or_else(|| StepResult {
        kind: step.kind.clone(),
        success: false,
        message: "replay produced no result".to_string(),
        output_path: None,
        duration_ms: 0,
    });
    session.step_results.push(r.clone());
    if r.success {
        session.step_index = (step_index as i64) + 1;
        session.final_output = r.output_path.clone();
    }
    update_debug_session(db, &session)?;
    Ok(session)
}

/// Replay every step from `from_index` to the end and persist the
/// updated session. Used by the "Run from Here" debugger button.
pub fn run_from_here(
    db: &Database,
    session_id: i64,
    from_index: i64,
    working_dir: &Path,
) -> Result<DebugSession, String> {
    let mut session = get_debug_session(db, session_id)?;
    if from_index < 0 || (from_index as usize) > session.steps.len() {
        return Err("from_index out of range".to_string());
    }
    session.step_results.truncate(from_index as usize);
    session.step_index = from_index;

    let input_pdf: std::path::PathBuf = if (from_index as usize) == 0 {
        std::path::PathBuf::from(&session.pdf_path)
    } else if let Some(prev) = session
        .step_results
        .last()
        .and_then(|r| r.output_path.clone())
    {
        std::path::PathBuf::from(prev)
    } else {
        std::path::PathBuf::from(&session.pdf_path)
    };

    let steps_to_run: Vec<ActionStep> = session.steps[(from_index as usize)..].to_vec();
    let result: ReplayResult =
        crate::pdf::action_list::replay(&input_pdf, &steps_to_run, working_dir)?;
    session.step_results.extend(result.steps);
    session.final_output = result.final_output;
    session.step_index = (from_index + steps_to_run.len() as i64).min(session.steps.len() as i64);
    update_debug_session(db, &session)?;
    Ok(session)
}

/// Render the first page of `pdf_path` to a PNG so the debugger UI can
/// show a thumbnail. Pass an optional `PdfEngine` (typically the
/// managed one from `lib.rs`); when `None` the function returns an
/// error.
pub fn render_first_page_thumbnail(
    engine: Option<&crate::pdf::engine::PdfEngine>,
    pdf_path: &Path,
    out_path: &Path,
    width_px: u32,
) -> Result<(), String> {
    let engine = match engine {
        Some(e) if e.is_available() => e,
        _ => return Err("PDF engine not available for thumbnail".to_string()),
    };
    let doc = engine.open_document(&pdf_path.to_string_lossy())?;
    let page = doc
        .pages()
        .get(0)
        .map_err(|e| format!("page 0: {e}"))?;
    let config = pdfium_render::prelude::PdfRenderConfig::new()
        .set_target_width(width_px as i32);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("render: {e}"))?;
    let w = bitmap.width() as u32;
    let h = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    if bytes.len() < (w as usize) * (h as usize) * 4 {
        return Err("rendered bitmap shorter than expected".to_string());
    }
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            img.put_pixel(
                x,
                y,
                image::Rgba([
                    bytes[i + 2],
                    bytes[i + 1],
                    bytes[i],
                    bytes[i + 3],
                ]),
            );
        }
    }
    img.save(out_path).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

/// Write a human-readable debug report PDF using `printpdf`. The
/// report includes a header (name + paths), a per-step table (kind,
/// params, result, duration, marker), and the original / final byte
/// size delta.
pub fn export_debug_report(
    session: &DebugSession,
    output_path: &Path,
) -> Result<(), String> {
    let (doc, page1, layer1) =
        PdfDocument::new(&session.name, Mm(210.0), Mm(297.0), "header");
    let font: IndirectFontRef = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("font: {e}"))?;
    let bold: IndirectFontRef = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("bold font: {e}"))?;
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let mut y = Mm(280.0);
    let left = Mm(15.0);
    let line_h = Mm(6.0);

    let _black = Rgb::new(0.0, 0.0, 0.0, None);
    let _grey = Rgb::new(0.4, 0.4, 0.4, None);
    let _red = Rgb::new(0.7, 0.0, 0.0, None);
    let _green = Rgb::new(0.0, 0.5, 0.0, None);

    write_line(&current_layer, &bold, 18.0, left, y,
        &format!("Action List Debug Report - {}", session.name));
    y -= line_h * 1.5;
    write_line(&current_layer, &font, 10.0, left, y,
        &format!("PDF: {}", truncate(&session.pdf_path, 80)));
    y -= line_h;
    write_line(&current_layer, &font, 10.0, left, y,
        &format!("Created: {}  Updated: {}  Step {}/{}",
            session.created_at, session.updated_at,
            session.step_index, session.steps.len()));
    y -= line_h * 1.5;

    let original_size = std::fs::metadata(&session.pdf_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let final_size = session
        .final_output
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);
    write_line(&current_layer, &font, 11.0, left, y,
        &format!("Original size: {} bytes  Final size: {} bytes  Delta: {:+} bytes",
            original_size, final_size,
            final_size as i64 - original_size as i64));
    y -= line_h * 1.5;

    write_line(&current_layer, &bold, 12.0, left, y, "Steps");
    y -= line_h;

    for (i, step) in session.steps.iter().enumerate() {
        if y.0 < 25.0 {
            let (np, nl) = doc.add_page(Mm(210.0), Mm(297.0), "step");
            let layer = doc.get_page(np).get_layer(nl);
            write_line(&layer, &bold, 14.0, left, y, "Steps (continued)");
            break;
        }
        let label = step.label.clone().unwrap_or_else(|| step.kind.clone());
        let params_str = serde_json::to_string(&step.params).unwrap_or_default();
        let result_str = session
            .step_results
            .get(i)
            .map(|r| {
                if r.success {
                    format!("OK ({} ms) -> {}",
                        r.duration_ms,
                        r.output_path.as_deref().unwrap_or("-"))
                } else {
                    format!("FAIL - {}", r.message)
                }
            })
            .unwrap_or_else(|| "(not yet executed)".to_string());
        let marker = if (i as i64) < session.step_index {
            "[done]"
        } else if (i as i64) == session.step_index {
            "[next]"
        } else {
            "[todo]"
        };
        write_line(&current_layer, &bold, 10.5, left, y,
            &format!("{} [{}/{}] {} - {}",
                marker, i + 1, session.steps.len(), label, step.kind));
        y -= line_h;
        write_line(&current_layer, &font, 9.0, left + Mm(5.0), y,
            &format!("    params: {}", truncate(&params_str, 90)));
        y -= line_h;
        write_line(&current_layer, &font, 9.0, left + Mm(5.0), y,
            &format!("    result: {}", truncate(&result_str, 90)));
        y -= line_h * 1.1;
    }

    let bytes = doc
        .save_to_bytes()
        .map_err(|e| format!("save_to_bytes: {e}"))?;
    std::fs::write(output_path, bytes).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

fn write_line(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    size: f32,
    x: Mm,
    y: Mm,
    text: &str,
) {
    layer.use_text(text, size, x, y, font);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        let take = max.saturating_sub(3);
        let boundary = s
            .char_indices()
            .map(|(i, _)| i)
            .filter(|&i| i <= take)
            .last()
            .unwrap_or(0);
        format!("{}...", &s[..boundary])
    } else {
        s.chars().take(max).collect()
    }
}

pub fn pdf_size(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

pub fn is_valid_pdf(p: &Path) -> bool {
    Document::load(p).map(|d| !d.get_pages().is_empty()).unwrap_or(false)
}

#[allow(dead_code)]
fn _unused_pdfdocref(_d: &PdfDocumentReference) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn truncate_long() {
        let s = "a".repeat(100);
        let out = truncate(&s, 10);
        assert!(out.len() <= 10);
    }

    #[test]
    fn pdf_size_missing_is_zero() {
        let p = std::path::Path::new("/no/such/file.pdf");
        assert_eq!(pdf_size(p), 0);
    }
}
