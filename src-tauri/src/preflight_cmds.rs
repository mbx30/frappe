//! Preflight checking commands (Phase 4.1–4.5).
//!
//! PDF preflight checks, profiles, action lists, batch processing,
//! hot folders, barcode detection, and redaction.

use tauri::State;

use crate::db::Database;
use crate::models::*;
use crate::pdf::bleed::BleedFinding;
use crate::pdf::boxes::PageBoxFinding;
use crate::pdf::color::{ColorSpaceFinding, InkCoverageFinding, SpotColorFinding};
use crate::pdf::fonts::FontFinding;
use crate::pdf::images::ImageResolutionFinding;
use crate::pdf::metadata::OutputIntent;
use crate::pdf::overprint::{HiddenContentFinding, OverprintFinding, TransparencyFinding};
use crate::pdf::pdfx::PdfXFinding;
use crate::pdf::redact::{RedactionRect, RedactionResult};
use crate::pdf::security::SecurityFinding;
use crate::pdf::transforms::{ConversionResult, IccProfileInfo};
use crate::security;

/// Convert a 0-based page index to the 1-based lopdf page ID.
fn lopdf_page_id(page_index: usize) -> u32 {
    (page_index + 1) as u32
}

// ── Preflight checks ───────────────────────────────────────────────────

#[tauri::command]
pub fn check_fonts(path: String) -> Result<Vec<FontFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::fonts::collect_fonts(&doc))
}

#[tauri::command]
pub fn check_page_boxes(path: String) -> Result<Vec<PageBoxFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::boxes::check_page_boxes(&doc))
}

#[tauri::command]
pub fn check_image_resolution(path: String) -> Result<Vec<ImageResolutionFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::images::check_image_resolution(&doc))
}

#[tauri::command]
pub fn check_bleed(path: String, min_bleed_mm: Option<f64>) -> Result<Vec<BleedFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let min = min_bleed_mm.unwrap_or(3.0);
    Ok(crate::pdf::bleed::check_bleed(&doc, min))
}

#[tauri::command]
pub fn add_bleed(
    path: String,
    amount_mm: f64,
    output_path: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    if amount_mm < 0.0 {
        return Err("amount_mm must be non-negative".to_string());
    }
    let mut doc = lopdf::Document::load(&path)
        .map_err(|e| format!("Failed to open PDF: {}", e))?;
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

    let mut pages_with_bleed = 0usize;
    for obj_id in &page_ids {
        let page_dict = doc
            .get_dictionary_mut(*obj_id)
            .map_err(|e| format!("Failed to get page dict: {}", e))?;

        let rotate = page_dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        if !matches!(rotate, 0 | 90 | 180 | 270) {
            return Err(format!(
                "Page {:?} has unsupported Rotate value {rotate}; expected 0/90/180/270",
                obj_id
            ));
        }

        let bleed_vals = get_array_vals(page_dict, b"BleedBox");
        let new_bleed = if let Some(bb) = bleed_vals {
            vec![
                bb[0] - amount_pts,
                bb[1] - amount_pts,
                bb[2] + amount_pts,
                bb[3] + amount_pts,
            ]
        } else if let Some(trim) = get_array_vals(page_dict, b"TrimBox") {
            vec![
                trim[0] - amount_pts,
                trim[1] - amount_pts,
                trim[2] + amount_pts,
                trim[3] + amount_pts,
            ]
        } else {
            continue;
        };

        page_dict.set(
            "BleedBox",
            lopdf::Object::Array(vec![
                lopdf::Object::Real(new_bleed[0] as f32),
                lopdf::Object::Real(new_bleed[1] as f32),
                lopdf::Object::Real(new_bleed[2] as f32),
                lopdf::Object::Real(new_bleed[3] as f32),
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
                    lopdf::Object::Array(vec![
                        lopdf::Object::Real(new_media[0] as f32),
                        lopdf::Object::Real(new_media[1] as f32),
                        lopdf::Object::Real(new_media[2] as f32),
                        lopdf::Object::Real(new_media[3] as f32),
                    ]),
                );
            }
        }
        pages_with_bleed += 1;
    }
    if pages_with_bleed == 0 {
        return Err(
            "No pages had a BleedBox or TrimBox to expand; nothing written".to_string(),
        );
    }
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn check_output_intents(path: String) -> Result<Vec<OutputIntent>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::metadata::get_output_intents(&doc))
}

#[tauri::command]
pub fn check_security(path: String) -> Result<Vec<SecurityFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
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
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;

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
pub fn check_color_spaces(
    path: String,
    target_profile: String,
) -> Result<Vec<ColorSpaceFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_color_spaces(&doc, &target_profile))
}

#[tauri::command]
pub fn check_overprint(path: String) -> Result<Vec<OverprintFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_overprint(&doc))
}

#[tauri::command]
pub fn check_transparency(path: String) -> Result<Vec<TransparencyFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_transparency(&doc))
}

#[tauri::command]
pub fn check_hidden_content(path: String) -> Result<Vec<HiddenContentFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::overprint::check_hidden_content(&doc))
}

#[tauri::command]
pub fn check_spot_colors(path: String) -> Result<Vec<SpotColorFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_spot_colors(&doc))
}

#[tauri::command]
pub fn check_ink_coverage(path: String) -> Result<Vec<InkCoverageFinding>, String> {
    let _path = security::validate_read_path(&path)?;
    let doc = lopdf::Document::load(&_path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    Ok(crate::pdf::color::check_ink_coverage(&doc))
}

#[tauri::command]
pub fn list_icc_profiles() -> Vec<IccProfileInfo> {
    crate::pdf::transforms::get_bundled_icc_profiles()
}

#[tauri::command]
#[allow(unused_variables)]
pub async fn convert_rgb_to_cmyk(
    path: String,
    output_path: String,
    scope: Option<String>,
    src_profile: Option<String>,
    dst_profile: Option<String>,
    rendering_intent: Option<String>,
) -> Result<ConversionResult, String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<ConversionResult, String> {
        let mut doc =
            lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
        let scope = scope.as_deref().unwrap_or("both");
        let result = crate::pdf::transforms::convert_rgb_to_cmyk(&mut doc, scope)?;
        doc.save(&output_path)
            .map_err(|e| format!("Failed to save converted PDF: {}", e))?;
        Ok(result)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

#[tauri::command]
pub fn add_output_intent(
    path: String,
    output_path: String,
    icc_profile: String,
    condition_id: String,
    condition: String,
) -> Result<(), String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;
    let icc_profile = security::validate_read_path(&icc_profile)?;
    let mut doc = lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
    let icc_data =
        std::fs::read(&icc_profile).map_err(|e| format!("Failed to read ICC profile: {}", e))?;
    crate::pdf::transforms::add_output_intent(&mut doc, &icc_data, &condition_id, &condition)?;
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(())
}

// ── Preflight profiles and findings ────────────────────────────────────

#[tauri::command]
pub fn save_preflight_run(
    db: State<'_, Database>,
    job_id: i64,
    profile: String,
    findings: Vec<PreflightFindingInput>,
) -> Result<i64, String> {
    db.save_preflight_run(job_id, &profile, &findings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_preflight_runs(
    db: State<'_, Database>,
    job_id: i64,
) -> Result<Vec<PreflightRunSummary>, String> {
    db.list_preflight_runs(job_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_findings_for_run(
    db: State<'_, Database>,
    run_id: i64,
) -> Result<Vec<PreflightFinding>, String> {
    db.list_findings_for_run(run_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_check_registry() -> Vec<crate::pdf::registry::CheckDefinition> {
    crate::pdf::registry::CHECK_REGISTRY.to_vec()
}

#[tauri::command]
pub async fn run_profile(
    db: State<'_, Database>,
    profile_id: i64,
    path: String,
) -> Result<crate::pdf::registry::RunProfileResult, String> {
    let profile = db
        .get_preflight_profile(profile_id)
        .map_err(|e| e.to_string())?;
    let path = security::validate_read_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<crate::pdf::registry::RunProfileResult, String> {
        let doc =
            lopdf::Document::load(&path).map_err(|e| format!("Failed to open PDF: {}", e))?;
        let mut findings: Vec<String> = Vec::new();

        let name_lower = profile.name.to_lowercase();
        if name_lower.contains("pdf/x-1a") {
            let f = crate::pdf::pdfx::check_pdfx(&doc, "PDF/X-1a:2003");
            findings.extend(f.iter().map(|x| x.message.clone()));
        } else if name_lower.contains("pdf/x-4") {
            let f = crate::pdf::pdfx::check_pdfx(&doc, "PDF/X-4");
            findings.extend(f.iter().map(|x| x.message.clone()));
        }
        let cs = crate::pdf::color::check_color_spaces(&doc, "Coated FOGRA39");
        findings.extend(cs.iter().map(|x| x.message.clone()));
        let sp = crate::pdf::color::check_spot_colors(&doc);
        findings.extend(sp.iter().map(|x| x.message.clone()));
        let ic = crate::pdf::color::check_ink_coverage(&doc);
        findings.extend(ic.iter().map(|x| x.message.clone()));

        Ok(crate::pdf::registry::RunProfileResult {
            profile_name: profile.name,
            findings_count: findings.len(),
        })
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

#[tauri::command]
pub fn create_preflight_profile(
    db: State<'_, Database>,
    input: PreflightProfileInput,
) -> Result<PreflightProfile, String> {
    db.create_preflight_profile(&input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_preflight_profiles(db: State<'_, Database>) -> Result<Vec<PreflightProfile>, String> {
    db.list_preflight_profiles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_preflight_profile(db: State<'_, Database>, id: i64) -> Result<PreflightProfile, String> {
    db.get_preflight_profile(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_preflight_profile(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_preflight_profile(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_profile_checks(
    db: State<'_, Database>,
    profile_id: i64,
) -> Result<Vec<ProfileCheck>, String> {
    db.list_profile_checks(profile_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_check(
    db: State<'_, Database>,
    check_id: i64,
    enabled: bool,
    severity: String,
) -> Result<(), String> {
    db.update_profile_check(check_id, enabled, &severity)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_profile_fixups(
    db: State<'_, Database>,
    profile_id: i64,
) -> Result<Vec<ProfileFixup>, String> {
    db.list_profile_fixups(profile_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile_fixup(
    db: State<'_, Database>,
    fixup_id: i64,
    enabled: bool,
    params: String,
) -> Result<(), String> {
    db.update_profile_fixup(fixup_id, enabled, &params)
        .map_err(|e| e.to_string())
}

// ── Export ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn generate_approval_sheet(
    info: crate::pdf::approval_sheet::ApprovalSheetInfo,
    output_path: String,
) -> Result<(), String> {
    let output_path = security::validate_write_path(&output_path)?;
    crate::pdf::approval_sheet::generate_approval_sheet(&info, &output_path.to_string_lossy())
}

#[tauri::command]
pub fn export_preflight_report_json(
    db: State<'_, Database>,
    run_id: i64,
) -> Result<serde_json::Value, String> {
    let findings = db
        .list_findings_for_run(run_id)
        .map_err(|e| e.to_string())?;
    let rows: Vec<serde_json::Value> = findings
        .into_iter()
        .map(|f| {
            serde_json::json!({
                "check_name": f.check_name,
                "severity": f.severity,
                "page_num": f.page_num,
                "message": f.message,
                "fix_hint": f.fix_hint,
            })
        })
        .collect();
    Ok(serde_json::json!({ "findings": rows }))
}

#[tauri::command]
pub fn export_preflight_report_csv(db: State<'_, Database>, run_id: i64) -> Result<String, String> {
    let findings = db
        .list_findings_for_run(run_id)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::<u8>::new();
    {
        let mut wtr = csv::Writer::from_writer(&mut out);
        wtr.write_record(["check_name", "severity", "page_num", "message", "fix_hint"])
            .map_err(|e| e.to_string())?;
        for f in findings {
            let page_num = f.page_num.map(|n| n.to_string()).unwrap_or_default();
            wtr.write_record([&f.check_name, &f.severity, &page_num, &f.message, &f.fix_hint])
                .map_err(|e| e.to_string())?;
        }
        wtr.flush().map_err(|e| e.to_string())?;
    }
    let mut s = String::from_utf8(out).map_err(|e| e.to_string())?;
    if s.ends_with("\r\n") {
        s.truncate(s.len() - 2);
    } else if s.ends_with('\n') {
        s.truncate(s.len() - 1);
    }
    Ok(s)
}

// ── Action Lists (#38) ────────────────────────────────────────────────

#[tauri::command]
pub fn create_action_list(
    db: State<'_, Database>,
    input: ActionListInput,
) -> Result<ActionList, String> {
    db.create_action_list(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_action_lists(db: State<'_, Database>) -> Result<Vec<ActionList>, String> {
    db.list_action_lists().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_action_list(db: State<'_, Database>, id: i64) -> Result<ActionList, String> {
    db.get_action_list(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_action_list(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_action_list(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_action_list_step(
    db: State<'_, Database>,
    action_list_id: i64,
    input: ActionListStepInput,
) -> Result<ActionListStep, String> {
    db.add_action_list_step(action_list_id, &input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_action_list_steps(
    db: State<'_, Database>,
    action_list_id: i64,
) -> Result<Vec<ActionListStep>, String> {
    db.list_action_list_steps(action_list_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_action_list_step(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_action_list_step(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_action_list_steps(
    db: State<'_, Database>,
    action_list_id: i64,
    step_ids: Vec<i64>,
) -> Result<(), String> {
    db.reorder_action_list_steps(action_list_id, &step_ids)
        .map_err(|e| e.to_string())
}

// ── Action list record / replay ────────────────────────────────────────

#[tauri::command]
pub fn start_action_recording(name: String) -> Result<(), String> {
    crate::pdf::action_list::start_recording(name)
}

#[tauri::command]
pub fn record_action_step(
    step: crate::pdf::action_list::ActionStep,
) -> Result<(), String> {
    crate::pdf::action_list::record_step(step)
}

#[tauri::command]
pub fn stop_action_recording() -> Result<crate::pdf::action_list::ActionList, String> {
    crate::pdf::action_list::stop_recording()
}

#[tauri::command]
pub fn cancel_action_recording() -> Result<(), String> {
    crate::pdf::action_list::cancel_recording()
}

#[tauri::command]
pub fn is_action_recording() -> bool {
    crate::pdf::action_list::is_recording()
}

#[tauri::command]
pub fn replay_action_list(
    input_pdf: String,
    steps: Vec<crate::pdf::action_list::ActionStep>,
    working_dir: String,
) -> Result<crate::pdf::action_list::ReplayResult, String> {
    let input_pdf = security::validate_read_path(&input_pdf)?;
    let working_dir = security::validate_write_path(&working_dir)?;
    crate::pdf::action_list::replay(
        std::path::Path::new(&input_pdf),
        &steps,
        std::path::Path::new(&working_dir),
    )
}

// ── Action list debugger (#268) ────────────────────────────────────────

#[tauri::command]
pub fn create_debug_session(
    db: State<'_, Database>,
    name: String,
    pdf_path: String,
    steps: Vec<crate::pdf::action_list::ActionStep>,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    let pdf_path = security::validate_read_path(&pdf_path)?;
    let pdf_path_str = pdf_path.to_str().ok_or("path is not valid UTF-8")?;
    crate::pdf::action_list_debugger::create_debug_session(&db, &name, pdf_path_str, &steps)
}

#[tauri::command]
pub fn list_debug_sessions(
    db: State<'_, Database>,
) -> Result<Vec<crate::pdf::action_list_debugger::DebugSession>, String> {
    crate::pdf::action_list_debugger::list_debug_sessions(&db)
}

#[tauri::command]
pub fn get_debug_session(
    db: State<'_, Database>,
    id: i64,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    crate::pdf::action_list_debugger::get_debug_session(&db, id)
}

#[tauri::command]
pub fn delete_debug_session(db: State<'_, Database>, id: i64) -> Result<(), String> {
    crate::pdf::action_list_debugger::delete_debug_session(&db, id)
}

#[tauri::command]
pub fn step_forward_debug(
    db: State<'_, Database>,
    id: i64,
    working_dir: String,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    let working_dir = security::validate_write_path(&working_dir)?;
    crate::pdf::action_list_debugger::step_forward(
        &db,
        id,
        std::path::Path::new(&working_dir),
    )
}

#[tauri::command]
pub fn run_from_here_debug(
    db: State<'_, Database>,
    id: i64,
    from_index: i64,
    working_dir: String,
) -> Result<crate::pdf::action_list_debugger::DebugSession, String> {
    let working_dir = security::validate_write_path(&working_dir)?;
    crate::pdf::action_list_debugger::run_from_here(
        &db,
        id,
        from_index,
        std::path::Path::new(&working_dir),
    )
}

#[tauri::command]
pub fn render_debug_thumbnail(
    engine: State<'_, crate::pdf::engine::PdfEngine>,
    pdf_path: String,
    out_path: String,
    width_px: u32,
) -> Result<(), String> {
    let pdf_path = security::validate_read_path(&pdf_path)?;
    let out_path = security::validate_write_path(&out_path)?;
    crate::pdf::action_list_debugger::render_first_page_thumbnail(
        Some(&engine),
        std::path::Path::new(&pdf_path),
        std::path::Path::new(&out_path),
        width_px,
    )
}

#[tauri::command]
pub fn export_debug_report_pdf(
    db: State<'_, Database>,
    id: i64,
    output_path: String,
) -> Result<(), String> {
    let output_path = security::validate_write_path(&output_path)?;
    let session = crate::pdf::action_list_debugger::get_debug_session(&db, id)?;
    crate::pdf::action_list_debugger::export_debug_report(
        &session,
        std::path::Path::new(&output_path),
    )
}

// ── Hot Folders (#42) ──────────────────────────────────────────────────

#[tauri::command]
pub fn start_hot_folder_watcher(
    app_handle: tauri::AppHandle,
    config: crate::pdf::watcher::HotFolderConfig,
) -> Result<String, String> {
    security::validate_read_dir(&config.watch_path)?;
    security::validate_write_path(&config.output_path)?;
    crate::pdf::watcher::start_hot_folder_watcher(config, Some(app_handle))
}

#[tauri::command]
pub fn stop_hot_folder_watcher() -> Result<(), String> {
    crate::pdf::watcher::stop_hot_folder_watcher()
}

#[tauri::command]
pub fn create_hot_folder(
    db: State<'_, Database>,
    input: HotFolderInput,
) -> Result<HotFolder, String> {
    security::validate_read_dir(&input.watch_path)?;
    security::validate_write_path(&input.output_path)?;
    db.create_hot_folder(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_hot_folders(db: State<'_, Database>) -> Result<Vec<HotFolder>, String> {
    db.list_hot_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_hot_folder(db: State<'_, Database>, id: i64) -> Result<(), String> {
    db.delete_hot_folder(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_hot_folder(db: State<'_, Database>, id: i64, is_active: bool) -> Result<(), String> {
    db.toggle_hot_folder(id, is_active)
        .map_err(|e| e.to_string())
}

// ── Redaction (#231) ───────────────────────────────────────────────────

#[tauri::command]
pub fn redact_pdf(
    db: State<'_, Database>,
    path: String,
    output_path: String,
    redactions: Vec<RedactionRect>,
    operator_name: Option<String>,
    notes: Option<String>,
) -> Result<RedactionResult, String> {
    let path = security::validate_read_path(&path)?;
    let output_path = security::validate_write_path(&output_path)?;

    let input = std::fs::read(&path).map_err(|e| format!("Failed to read PDF: {e}"))?;
    let output_path_str = output_path.to_str().ok_or("output path is not valid UTF-8")?;
    let result = crate::pdf::redact::redact_pdf_content(&input, &redactions, output_path_str)?;

    let regions_json = serde_json::to_string(&redactions).unwrap_or_else(|_| "[]".to_string());
    let operator = operator_name.unwrap_or_default();
    let notes = notes.unwrap_or_default();
    let path_str = path.to_str().ok_or("path is not valid UTF-8")?;

    db.log_redaction_operation(
        path_str,
        &result.output_path,
        &result.content_hash,
        &regions_json,
        result.redactions_applied as i64,
        result.pages_modified as i64,
        &operator,
        &notes,
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn get_redaction_audit_log(
    db: State<'_, Database>,
    path: String,
) -> Result<Vec<RedactionAuditEntry>, String> {
    db.query_redaction_log(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn verify_redaction_chain(db: State<'_, Database>, path: String) -> Result<bool, String> {
    db.verify_redaction_chain_integrity(&path)
        .map_err(|e| e.to_string())
}

// ── Batch processing (#40) ─────────────────────────────────────────────

#[tauri::command]
pub fn create_batch_job(
    db: State<'_, Database>,
    action_list_id: i64,
    files: Vec<String>,
) -> Result<BatchJob, String> {
    db.create_batch_job(action_list_id, &files)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_batch_jobs(db: State<'_, Database>) -> Result<Vec<BatchJob>, String> {
    db.list_batch_jobs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_batch_job(db: State<'_, Database>, id: i64) -> Result<BatchJob, String> {
    db.get_batch_job(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_batch(db: State<'_, Database>, batch_id: i64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| -> Result<(), String> { Ok(()) })
        .await
        .map_err(|e| format!("spawn_blocking join error: {e}"))?
        .map_err(|e: String| e)?;
    db.run_batch(batch_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_batch_results(
    db: State<'_, Database>,
    batch_id: i64,
) -> Result<Vec<BatchResult>, String> {
    db.list_batch_results(batch_id).map_err(|e| e.to_string())
}

// ── Barcode detection (#270) ───────────────────────────────────────────

#[tauri::command]
pub fn detect_barcodes(
    engine: State<'_, crate::pdf::engine::PdfEngine>,
    path: String,
    page_index: usize,
) -> Result<Vec<crate::pdf::barcode::BarcodeDetection>, String> {
    let path = security::validate_read_path(&path)?;
    let path_str = path.to_str().ok_or("path is not valid UTF-8")?;
    use image::RgbaImage;
    let doc = engine.open_document(path_str)?;
    let idx: i32 = page_index
        .try_into()
        .map_err(|_| format!("Page index too large: {page_index}"))?;
    let page = doc
        .pages()
        .get(idx)
        .map_err(|e| format!("Page {page_index} not found: {e}"))?;
    let dpi = 200.0_f64;
    let page_width_pts = page.width().value as f64;
    let page_height_pts = page.height().value as f64;
    let target_w = ((page_width_pts * dpi / 72.0) as i32).max(64);
    let config = pdfium_render::prelude::PdfRenderConfig::new().set_target_width(target_w);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render error: {e}"))?;
    let pw = bitmap.width() as u32;
    let ph = bitmap.height() as u32;
    let bytes = bitmap.as_raw_bytes();
    if bytes.len() < (pw as usize) * (ph as usize) * 4 {
        return Err("Rendered bitmap shorter than expected".to_string());
    }
    let mut img = RgbaImage::new(pw, ph);
    for y in 0..ph {
        for x in 0..pw {
            let i = ((y * pw + x) * 4) as usize;
            img.put_pixel(
                x,
                y,
                image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]),
            );
        }
    }
    let mut rgba: Vec<u8> = Vec::with_capacity((pw as usize) * (ph as usize) * 4);
    for px in img.pixels() {
        rgba.extend_from_slice(&px.0);
    }
    let input = crate::pdf::barcode::BarcodeInputImage {
        pixels: rgba,
        width: pw,
        height: ph,
        page_width_pts,
        page_height_pts,
    };
    crate::pdf::barcode::detect_barcodes_in_image(&input)
}

// ── PDF Compression (#49) ──────────────────────────────────────────────

#[tauri::command]
pub async fn compress_pdf(
    path: String,
    output_path: Option<String>,
    options: Option<crate::pdf::compress::CompressionOptions>,
) -> Result<crate::pdf::compress::CompressionResult, String> {
    let path = security::validate_read_path(&path)?;
    let output_path = if let Some(out) = output_path {
        let validated = security::validate_write_path(&out)?;
        Some(validated)
    } else {
        None
    };
    let path_str = path.to_string_lossy().into_owned();
    let output_path_str: Option<String> = output_path.map(|p| p.to_string_lossy().into_owned());
    let opts = options.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        crate::pdf::compress::compress_pdf(&path_str, output_path_str.as_deref(), &opts)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {e}"))?
}
