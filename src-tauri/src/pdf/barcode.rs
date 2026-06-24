//! Barcode detection and validation (issue #270).
//!
//! Workflow:
//!   1. Render the page at 200 DPI via PDFium (handled by the caller).
//!   2. Build an `image::DynamicImage` from the RGBA pixel buffer.
//!   3. Pass it to `rxing::helpers::detect_multiple_in_image_with_hints`.
//!   4. Validate the quiet-zone and physical size against industry
//!      standards (Code128 10× narrow bar, QR 4× module, EAN-13 ≥26.73 mm).
//!
//! The `BarcodeDetection` return value carries the decoded text, the bbox
//! in page coordinates, and a per-barcode status (`ok` | `undersized` |
//! `tight_quiet_zone`).

use rxing::BarcodeFormat;
use rxing::DecodeHints;
use rxing::RXingResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeDetection {
    pub text: String,
    /// Format identifier (e.g. "QR_CODE", "CODE_128", "EAN_13").
    pub format: String,
    /// Bounding box in page-space points: [x_min, y_min, x_max, y_max].
    pub bbox: [f64; 4],
    pub orientation: i32,
    /// Status: "ok", "undersized", "tight_quiet_zone".
    pub status: String,
    /// Per-side quiet-zone in millimetres (top, right, bottom, left).
    pub quiet_zone_mm: [f64; 4],
    /// Physical size in millimetres (width, height).
    pub size_mm: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeInputImage {
    /// RGBA8 pixel data, 4 bytes per pixel.
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Page size in points (1/72 inch). Used to convert pixel coords to mm.
    pub page_width_pts: f64,
    pub page_height_pts: f64,
}

/// Minimum physical size for industry standards.
fn min_size_mm(format: &str) -> (f64, f64) {
    match format {
        "EAN_13" | "EAN_8" => (26.73, 18.28),
        "UPC_A" | "UPC_E" => (25.91, 18.28),
        "CODE_128" | "CODE_39" | "CODE_93" => (10.0, 19.0),
        "QR_CODE" => (10.0, 10.0),
        "DATA_MATRIX" => (10.0, 10.0),
        "PDF_417" => (20.0, 10.0),
        "ITF" => (20.0, 10.0),
        _ => (10.0, 10.0),
    }
}

fn required_quiet_zone_modules(format: &str) -> f64 {
    match format {
        "QR_CODE" => 4.0,
        "DATA_MATRIX" => 4.0,
        "CODE_128" | "CODE_39" | "CODE_93" => 10.0,
        "EAN_13" | "EAN_8" | "UPC_A" | "UPC_E" => 3.63 / 0.264,
        _ => 4.0,
    }
}

/// Decode a DynamicImage and return all detected barcodes with validation.
pub fn detect_barcodes_in_image(
    input: &BarcodeInputImage,
) -> Result<Vec<BarcodeDetection>, String> {
    if input.pixels.len() < (input.width as usize) * (input.height as usize) * 4 {
        return Err("Image data shorter than expected".to_string());
    }
    let rgba = image::RgbaImage::from_raw(input.width, input.height, input.pixels.clone())
        .ok_or_else(|| "Could not build DynamicImage from RGBA buffer".to_string())?;
    let dyn_img = image::DynamicImage::ImageRgba8(rgba);

    let mut hints = DecodeHints::default();
    let formats: HashSet<BarcodeFormat> = [
        BarcodeFormat::QR_CODE,
        BarcodeFormat::CODE_128,
        BarcodeFormat::CODE_39,
        BarcodeFormat::CODE_93,
        BarcodeFormat::EAN_13,
        BarcodeFormat::EAN_8,
        BarcodeFormat::UPC_A,
        BarcodeFormat::UPC_E,
        BarcodeFormat::ITF,
        BarcodeFormat::DATA_MATRIX,
        BarcodeFormat::PDF_417,
    ]
    .into_iter()
    .collect();
    hints.PossibleFormats = Some(formats);
    hints.TryHarder = Some(true);
    hints.CharacterSet = Some("UTF-8".to_string());

    // The high-level helpers::detect_multiple_in_image_with_hints takes a
    // DynamicImage by value, so we clone.
    let dyn_img_for_multi = dyn_img.clone();
    let results: Vec<RXingResult> = match rxing::helpers::detect_multiple_in_image_with_hints(
        dyn_img_for_multi,
        &mut hints,
    ) {
        Ok(v) => v,
        Err(_) => {
            // Fall back to single-barcode detection in case the multi
            // reader reports "NotFoundException" for a single isolated code.
            let mut single_hints = hints.clone();
            match rxing::helpers::detect_in_image_with_hints(dyn_img, None, &mut single_hints) {
                Ok(r) => vec![r],
                Err(_) => Vec::new(),
            }
        }
    };

    let mut out = Vec::with_capacity(results.len());
    let page_w_pts = input.page_width_pts;
    let page_h_pts = input.page_height_pts;
    let img_w = input.width as f64;
    let img_h = input.height as f64;
    let mm_per_pt = 25.4 / 72.0;

    for r in results {
        // Result points are stored on the result; access them via the
        // standard Rust getters.
        let points = r.getPoints();
        let pts: Vec<(f64, f64)> = points.iter().map(|p| (p.x as f64, p.y as f64)).collect();
        if pts.len() < 4 {
            continue;
        }
        let x_min_px = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max_px = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min_px = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max_px = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        // Pixel → page points (PDF convention: bottom-left origin).
        let x_min_pt = x_min_px / img_w * page_w_pts;
        let x_max_pt = x_max_px / img_w * page_w_pts;
        let y_min_pt = (img_h - y_max_px) / img_h * page_h_pts;
        let y_max_pt = (img_h - y_min_px) / img_h * page_h_pts;
        let width_pt = (x_max_pt - x_min_pt).abs();
        let height_pt = (y_max_pt - y_min_pt).abs();
        let width_mm = width_pt * mm_per_pt;
        let height_mm = height_pt * mm_per_pt;

        let format_clean = format!("{:?}", r.getBarcodeFormat());
        let format_clean = format_clean.replace("BarcodeFormat::", "");

        let (min_w, min_h) = min_size_mm(&format_clean);
        let size_ok = width_mm >= min_w && height_mm >= min_h;
        let mut status = if !size_ok {
            "undersized".to_string()
        } else {
            "ok".to_string()
        };

        let edge_pad_px = 4.0;
        let qz_top_mm = if y_min_px >= edge_pad_px {
            (y_min_px / img_h * page_h_pts) * mm_per_pt
        } else {
            0.0
        };
        let qz_bottom_mm = if (img_h - y_max_px) >= edge_pad_px {
            ((img_h - y_max_px) / img_h * page_h_pts) * mm_per_pt
        } else {
            0.0
        };
        let qz_left_mm = if x_min_px >= edge_pad_px {
            (x_min_px / img_w * page_w_pts) * mm_per_pt
        } else {
            0.0
        };
        let qz_right_mm = if (img_w - x_max_px) >= edge_pad_px {
            ((img_w - x_max_px) / img_w * page_w_pts) * mm_per_pt
        } else {
            0.0
        };
        let module_mm = (width_mm.min(height_mm)) / 21.0;
        let req_qz_modules = required_quiet_zone_modules(&format_clean);
        let min_qz_mm = req_qz_modules * module_mm;
        let quiet_ok = qz_top_mm >= min_qz_mm
            && qz_bottom_mm >= min_qz_mm
            && qz_left_mm >= min_qz_mm
            && qz_right_mm >= min_qz_mm;
        if !quiet_ok && status == "ok" {
            status = "tight_quiet_zone".to_string();
        }

        out.push(BarcodeDetection {
            text: r.getText().to_string(),
            format: format_clean,
            bbox: [x_min_pt, y_min_pt, x_max_pt, y_max_pt],
            orientation: 0,
            status,
            quiet_zone_mm: [qz_top_mm, qz_right_mm, qz_bottom_mm, qz_left_mm],
            size_mm: [width_mm, height_mm],
        });
    }
    Ok(out)
}
