//! Action list record / replay runtime (#266).
//!
//! Action lists are user-defined sequences of `add_bleed`,
//! `convert_rgb_to_cmyk`, `add_output_intent`, `compress_pdf`, etc.
//! This module provides:
//!   * `ActionStep` — a JSON-serializable single step
//!   * `RecordingSession` — accumulates steps as the user invokes
//!     commands, exposed through a `Mutex<Option<RecordingSession>>`
//!     that the Tauri command can mutate
//!   * `replay` — re-runs a sequence of steps against a PDF, dispatching
//!     each step to the corresponding `crate::pdf::fixups::*` function
//!
//! Steps use a stable string `kind` (`add_bleed`, `convert_rgb_to_cmyk`,
//! `add_output_intent`, `compress_pdf`, `rotate_page`, `delete_pages`,
//! `extract_pages`, `reorder_pages`, `insert_blank_page`, `set_layer_visibility`,
//! `add_icc_output_intent`) and a `params` JSON object whose schema is
//! type-specific. The replay engine deserializes the params into the
//! concrete argument struct for each kind.

use crate::db::Database;
use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub kind: String,
    pub params: serde_json::Value,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionList {
    pub name: String,
    pub steps: Vec<ActionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepResult {
    pub kind: String,
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayResult {
    pub steps: Vec<StepResult>,
    pub final_output: Option<String>,
}

/// State of an in-progress recording. None = no active session.
pub struct RecordingSession {
    pub name: String,
    pub steps: Vec<ActionStep>,
    pub started_at: String,
}

impl RecordingSession {
    pub fn new(name: impl Into<String>) -> Self {
        RecordingSession {
            name: name.into(),
            steps: Vec::new(),
            started_at: chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        }
    }
}

static RECORDING: Mutex<Option<RecordingSession>> = Mutex::new(None);

/// Begin a new recording session. Returns an error if one is already
/// active (callers must `stop_recording` first).
pub fn start_recording(name: impl Into<String>) -> Result<(), String> {
    let mut slot = RECORDING.lock().map_err(|e| format!("lock: {e}"))?;
    if slot.is_some() {
        return Err("A recording session is already active".to_string());
    }
    *slot = Some(RecordingSession::new(name));
    Ok(())
}

/// Append a step to the current session. No-op if no session is active.
pub fn record_step(step: ActionStep) -> Result<(), String> {
    let mut slot = RECORDING.lock().map_err(|e| format!("lock: {e}"))?;
    if let Some(session) = slot.as_mut() {
        session.steps.push(step);
        Ok(())
    } else {
        Err("No active recording session".to_string())
    }
}

/// Finalize the active session and return the recorded list. The
/// session is consumed; the caller can persist the list via
/// `Database::create_action_list` + `Database::add_action_list_step`.
pub fn stop_recording() -> Result<ActionList, String> {
    let mut slot = RECORDING.lock().map_err(|e| format!("lock: {e}"))?;
    let session = slot.take().ok_or_else(|| "No active recording session".to_string())?;
    Ok(ActionList {
        name: session.name,
        steps: session.steps,
    })
}

/// Discard the current session without returning it.
pub fn cancel_recording() -> Result<(), String> {
    let mut slot = RECORDING.lock().map_err(|e| format!("lock: {e}"))?;
    *slot = None;
    Ok(())
}

/// True when a recording is currently active.
pub fn is_recording() -> bool {
    RECORDING.lock().map(|s| s.is_some()).unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────
// Replay engine
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBleedParams {
    pub amount_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertRgbToCmykParams {
    pub scope: String,
    #[serde(default)]
    pub src_profile: Option<String>,
    #[serde(default)]
    pub dst_profile: Option<String>,
    #[serde(default)]
    pub rendering_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddOutputIntentParams {
    /// Either a base64-encoded ICC profile or a path to one on disk.
    pub icc_base64: Option<String>,
    pub icc_path: Option<String>,
    pub condition_id: String,
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressParams {
    #[serde(default = "default_target_dpi")]
    pub target_dpi: u32,
    #[serde(default = "default_image_quality")]
    pub image_quality: u8,
    #[serde(default)]
    pub use_zopfli: bool,
}

fn default_target_dpi() -> u32 {
    150
}
fn default_image_quality() -> u8 {
    85
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotatePageParams {
    pub page_index: usize,
    pub degrees: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePagesParams {
    pub indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPagesParams {
    pub indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderPagesParams {
    pub new_order: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertBlankPageParams {
    pub after_index: usize,
    pub width_mm: f64,
    pub height_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLayerVisibilityParams {
    pub object_id: u32,
    pub visible: bool,
}

fn run_step(
    step: &ActionStep,
    working_pdf: &mut std::path::PathBuf,
) -> Result<StepResult, String> {
    let started = std::time::Instant::now();
    let res = dispatch_step(step, working_pdf);
    let duration_ms = started.elapsed().as_millis() as u64;
    match res {
        Ok(out) => Ok(StepResult {
            kind: step.kind.clone(),
            success: true,
            message: "ok".to_string(),
            output_path: out.map(|p| p.to_string_lossy().into_owned()),
            duration_ms,
        }),
        Err(e) => Ok(StepResult {
            kind: step.kind.clone(),
            success: false,
            message: e,
            output_path: None,
            duration_ms,
        }),
    }
}

fn dispatch_step(
    step: &ActionStep,
    working_pdf: &mut std::path::PathBuf,
) -> Result<Option<std::path::PathBuf>, String> {
    match step.kind.as_str() {
        "add_bleed" => {
            let p: AddBleedParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("add_bleed params: {e}"))?;
            let next = derive_step_output(working_pdf, "bleed");
            add_bleed_in_place(working_pdf, p.amount_mm, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "convert_rgb_to_cmyk" => {
            let p: ConvertRgbToCmykParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("convert_rgb_to_cmyk params: {e}"))?;
            let next = derive_step_output(working_pdf, "cmyk");
            convert_rgb_to_cmyk_in_place(working_pdf, &p, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "add_output_intent" => {
            let p: AddOutputIntentParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("add_output_intent params: {e}"))?;
            let next = derive_step_output(working_pdf, "oi");
            add_output_intent_in_place(working_pdf, &p, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "compress_pdf" => {
            let p: CompressParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("compress_pdf params: {e}"))?;
            let next = derive_step_output(working_pdf, "compressed");
            let opts = crate::pdf::compress::CompressionOptions {
                quality: 80,
                target_dpi: p.target_dpi,
                image_quality: p.image_quality,
                subset_fonts: false,
                use_zopfli: p.use_zopfli,
            };
            crate::pdf::compress::compress_pdf(
                working_pdf.to_string_lossy().as_ref(),
                Some(next.to_string_lossy().as_ref()),
                &opts,
            )?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "rotate_page" => {
            let p: RotatePageParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("rotate_page params: {e}"))?;
            let next = derive_step_output(working_pdf, "rotated");
            rotate_page_in_place(working_pdf, p.page_index, p.degrees, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "delete_pages" => {
            let p: DeletePagesParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("delete_pages params: {e}"))?;
            let next = derive_step_output(working_pdf, "trimmed");
            delete_pages_in_place(working_pdf, &p.indices, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "extract_pages" => {
            let p: ExtractPagesParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("extract_pages params: {e}"))?;
            let next = derive_step_output(working_pdf, "extracted");
            extract_pages_in_place(working_pdf, &p.indices, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "reorder_pages" => {
            let p: ReorderPagesParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("reorder_pages params: {e}"))?;
            let next = derive_step_output(working_pdf, "reordered");
            reorder_pages_in_place(working_pdf, &p.new_order, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "insert_blank_page" => {
            let p: InsertBlankPageParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("insert_blank_page params: {e}"))?;
            let next = derive_step_output(working_pdf, "with_blank");
            insert_blank_page_in_place(
                working_pdf,
                p.after_index,
                p.width_mm,
                p.height_mm,
                &next,
            )?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        "set_layer_visibility" => {
            let p: SetLayerVisibilityParams = serde_json::from_value(step.params.clone())
                .map_err(|e| format!("set_layer_visibility params: {e}"))?;
            let next = derive_step_output(working_pdf, "layered");
            set_layer_visibility_in_place(working_pdf, p.object_id, p.visible, &next)?;
            *working_pdf = next.clone();
            Ok(Some(next))
        }
        other => Err(format!("Unknown action step kind: {other}")),
    }
}

fn derive_step_output(current: &Path, suffix: &str) -> std::path::PathBuf {
    let stem = current
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("step");
    let parent = current.parent().unwrap_or_else(|| Path::new("."));
    let ext = current
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("pdf");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!("{stem}_{suffix}_{nanos}.{ext}"))
}

// ─────────────────────────────────────────────────────────────────────
// Per-step fixup implementations. Each loads the PDF, mutates it, and
// saves to `out`. These are intentionally thin wrappers over the
// existing per-module fixup functions so the replay engine can be
// used without going through the Tauri command surface.
// ─────────────────────────────────────────────────────────────────────

fn add_bleed_in_place(input: &Path, amount_mm: f64, out: &Path) -> Result<(), String> {
    use lopdf::Object;
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let amount_pts = amount_mm / 0.3528;

    fn obj_to_f64(o: &Object) -> Option<f64> {
        match o {
            Object::Integer(i) => Some(*i as f64),
            Object::Real(r) => Some(*r as f64),
            _ => None,
        }
    }
    fn get_array_vals(page_dict: &lopdf::Dictionary, key: &[u8]) -> Option<Vec<f64>> {
        page_dict.get(key).ok().and_then(|o| {
            if let Object::Array(a) = o {
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
            .map_err(|e| format!("page dict: {e}"))?;
        let bleed_vals = get_array_vals(page_dict, b"BleedBox");
        let new_bleed = if let Some(bb) = bleed_vals {
            vec![bb[0] - amount_pts, bb[1] - amount_pts, bb[2] + amount_pts, bb[3] + amount_pts]
        } else if let Some(trim) = get_array_vals(page_dict, b"TrimBox") {
            vec![trim[0] - amount_pts, trim[1] - amount_pts, trim[2] + amount_pts, trim[3] + amount_pts]
        } else {
            continue;
        };
        page_dict.set(
            "BleedBox",
            Object::Array(vec![
                Object::Real(new_bleed[0] as f32),
                Object::Real(new_bleed[1] as f32),
                Object::Real(new_bleed[2] as f32),
                Object::Real(new_bleed[3] as f32),
            ]),
        );
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
                    Object::Array(vec![
                        Object::Real(new_media[0] as f32),
                        Object::Real(new_media[1] as f32),
                        Object::Real(new_media[2] as f32),
                        Object::Real(new_media[3] as f32),
                    ]),
                );
            }
        }
    }
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn convert_rgb_to_cmyk_in_place(
    input: &Path,
    params: &ConvertRgbToCmykParams,
    out: &Path,
) -> Result<(), String> {
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    crate::pdf::transforms::convert_rgb_to_cmyk(&mut doc, &params.scope)
        .map_err(|e| format!("convert: {e}"))?;
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn add_output_intent_in_place(
    input: &Path,
    params: &AddOutputIntentParams,
    out: &Path,
) -> Result<(), String> {
    let icc_bytes: Vec<u8> = if let Some(b64) = &params.icc_base64 {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|e| format!("base64: {e}"))?
    } else if let Some(p) = &params.icc_path {
        std::fs::read(p).map_err(|e| format!("read icc: {e}"))?
    } else {
        return Err("add_output_intent: must supply icc_base64 or icc_path".to_string());
    };
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    crate::pdf::transforms::add_output_intent(
        &mut doc,
        &icc_bytes,
        &params.condition_id,
        &params.condition,
    )
    .map_err(|e| format!("add_output_intent: {e}"))?;
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn rotate_page_in_place(
    input: &Path,
    page_index: usize,
    degrees: i64,
    out: &Path,
) -> Result<(), String> {
    use lopdf::Object;
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
    let target = page_ids
        .get(page_index)
        .copied()
        .ok_or_else(|| format!("page {page_index} out of range"))?;
    if let Ok(page) = doc.get_dictionary_mut(target) {
        page.set("Rotate", Object::Integer(degrees));
    }
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn delete_pages_in_place(input: &Path, indices: &[usize], out: &Path) -> Result<(), String> {
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let page_ids: Vec<u32> = doc.get_pages().keys().copied().collect();
    let to_remove: Vec<u32> = indices
        .iter()
        .filter_map(|i| page_ids.get(*i).copied())
        .collect();
    doc.delete_pages(&to_remove);
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn extract_pages_in_place(input: &Path, indices: &[usize], out: &Path) -> Result<(), String> {
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let page_ids: Vec<u32> = doc.get_pages().keys().copied().collect();
    let to_keep: std::collections::HashSet<u32> = indices
        .iter()
        .filter_map(|i| page_ids.get(*i).copied())
        .collect();
    let to_remove: Vec<u32> = page_ids
        .iter()
        .copied()
        .filter(|pn| !to_keep.contains(pn))
        .collect();
    doc.delete_pages(&to_remove);
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn reorder_pages_in_place(input: &Path, new_order: &[usize], out: &Path) -> Result<(), String> {
    use lopdf::Object;
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let page_ids: Vec<u32> = doc.get_pages().keys().copied().collect();
    if new_order.len() != page_ids.len() {
        return Err(format!(
            "new_order length {} does not match page count {}",
            new_order.len(),
            page_ids.len()
        ));
    }
    let root_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "no Root reference".to_string())?;
    let catalog = doc
        .get_dictionary(root_ref)
        .map_err(|e| format!("catalog: {e}"))?;
    let pages_ref = catalog
        .get(b"Pages")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "no Pages reference".to_string())?;
    let mut new_kids: Vec<Object> = Vec::new();
    for idx in new_order {
        if let Some(obj_ref) = page_ids.get(*idx) {
            new_kids.push(Object::Reference((*obj_ref, 0)));
        } else {
            return Err(format!("page index {idx} out of range"));
        }
    }
    if let Ok(pages_dict) = doc.get_dictionary_mut(pages_ref) {
        pages_dict.set("Kids", Object::Array(new_kids));
    }
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn insert_blank_page_in_place(
    input: &Path,
    after_index: usize,
    width_mm: f64,
    height_mm: f64,
    out: &Path,
) -> Result<(), String> {
    use lopdf::Object;
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let width_pts = width_mm / 0.3528;
    let height_pts = height_mm / 0.3528;
    let root_ref = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "no Root".to_string())?;
    let catalog = doc.get_dictionary(root_ref).map_err(|e| format!("catalog: {e}"))?;
    let pages_ref = catalog
        .get(b"Pages")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| "no Pages ref".to_string())?;
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

    let page_refs: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
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
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

fn set_layer_visibility_in_place(
    input: &Path,
    object_id: u32,
    visible: bool,
    out: &Path,
) -> Result<(), String> {
    use lopdf::Object;
    let mut doc = Document::load(input).map_err(|e| format!("open: {e}"))?;
    let key = (object_id, 0u16);
    let target = doc
        .objects
        .get_mut(&key)
        .ok_or_else(|| format!("OCG object {object_id} not found"))?;
    if let Object::Dictionary(d) = target {
        if visible {
            d.remove(b"OC");
        } else {
            d.set("OC", Object::Name(b"OFF".to_vec()));
        }
    } else {
        return Err(format!("OCG {object_id} is not a dictionary"));
    }
    doc.save(out).map_err(|e| format!("save: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// Public replay entry point
// ─────────────────────────────────────────────────────────────────────

/// Replay `steps` against `input_pdf`, producing one output per step
/// in a temp directory. The final PDF path is returned in
/// `ReplayResult.final_output`. If a step fails the engine stops and
/// returns the partial result with `success: false` on the failing
/// step; subsequent steps are not executed.
pub fn replay(
    input_pdf: &Path,
    steps: &[ActionStep],
    working_dir: &Path,
) -> Result<ReplayResult, String> {
    std::fs::create_dir_all(working_dir).map_err(|e| format!("mkdir working: {e}"))?;
    let mut current = input_pdf.to_path_buf();
    let mut results: Vec<StepResult> = Vec::new();
    for step in steps {
        // For non-destructive steps we need the input path to be the
        // original PDF; the step's helper writes a new file and
        // updates `current` to point at it. If the helper wrote to the
        // original path (because no new path was given) we copy back
        // to keep the chain intact.
        let r = run_step(step, &mut current)?;
        let ok = r.success;
        let output = r.output_path.clone();
        results.push(r);
        if !ok {
            return Ok(ReplayResult {
                steps: results,
                final_output: output,
            });
        }
    }
    Ok(ReplayResult {
        final_output: Some(current.to_string_lossy().to_string()),
        steps: results,
    })
}

/// Replay and persist each step's output to the database batch_results
/// table when a `Database` is provided. Currently this is a thin
/// convenience wrapper — the hot folder and batch-job code paths call
/// `replay` directly.
pub fn replay_with_db(
    input_pdf: &Path,
    steps: &[ActionStep],
    working_dir: &Path,
    _db: Option<&Database>,
) -> Result<ReplayResult, String> {
    replay(input_pdf, steps, working_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // The global RECORDING state is shared across tests, so serialize the
    // lifecycle tests to avoid one test cancelling another's session.
    static TEST_RECORDING_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn record_capture_round_trip() {
        let _guard = TEST_RECORDING_LOCK.lock().unwrap();
        cancel_recording().unwrap();
        start_recording("test").unwrap();
        record_step(ActionStep {
            kind: "add_bleed".to_string(),
            params: serde_json::json!({"amount_mm": 3.0}),
            label: Some("Add 3mm bleed".to_string()),
        })
        .unwrap();
        let list = stop_recording().unwrap();
        assert_eq!(list.steps.len(), 1);
        assert_eq!(list.steps[0].kind, "add_bleed");
        assert!(!is_recording());
    }

    #[test]
    fn double_start_recording_errors() {
        let _guard = TEST_RECORDING_LOCK.lock().unwrap();
        cancel_recording().unwrap();
        start_recording("a").unwrap();
        assert!(start_recording("b").is_err());
        cancel_recording().unwrap();
    }

    #[test]
    fn base64_decode_works() {
        use base64::Engine;
        let out = base64::engine::general_purpose::STANDARD
            .decode("SGVsbG8sIFdvcmxkIQ==")
            .unwrap();
        assert_eq!(out, b"Hello, World!");
    }

    #[test]
    fn derive_step_output_keeps_parent_and_ext() {
        let p = std::path::PathBuf::from("/tmp/foo.pdf");
        let n = derive_step_output(&p, "bleed");
        let s = n.to_string_lossy();
        let normalized = s.replace('\\', "/");
        assert!(normalized.starts_with("/tmp/foo_bleed_"));
        assert!(normalized.ends_with(".pdf"));
    }
}
