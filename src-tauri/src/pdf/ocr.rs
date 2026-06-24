// OCR (Optical Character Recognition) for converting scanned PDFs to searchable text.
// Issue #229: Implement OCR with text detection, backend selection, and hidden text layer overlay.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// OCR backend selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OcrBackend {
    /// Local Tesseract OCR engine (requires tesseract binary installed).
    Tesseract,
    /// Google Cloud Vision API (requires API key in settings).
    GoogleCloudVision,
}

impl OcrBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            OcrBackend::Tesseract => "tesseract",
            OcrBackend::GoogleCloudVision => "google_cloud_vision",
        }
    }
}

/// Detected text from a single page via OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPageResult {
    /// Page number (0-based index).
    pub page_index: usize,
    /// Extracted text from the page.
    pub text: String,
    /// Confidence score (0.0 to 1.0), if available.
    pub confidence: f32,
    /// Bounding boxes for each detected text region, if available.
    pub regions: Vec<OcrTextRegion>,
}

/// A single text region detected on a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTextRegion {
    /// Detected text.
    pub text: String,
    /// Bounding box: (left, top, width, height) in PDF coordinates.
    pub bbox: (f32, f32, f32, f32),
    /// Confidence score for this region.
    pub confidence: f32,
}

/// Result of running OCR on a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// Pages that were processed.
    pub pages: Vec<OcrPageResult>,
    /// Total text extracted.
    pub total_text: String,
    /// Backend used.
    pub backend: String,
    /// Number of pages processed.
    pub pages_processed: usize,
    /// Time taken (milliseconds).
    pub duration_ms: u64,
}

/// Options for running OCR on a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrOptions {
    /// Pages to process. If empty, process all pages. (0-based indices)
    pub pages: Vec<usize>,
    /// OCR backend to use.
    pub backend: OcrBackend,
    /// Whether to overlay the OCR text as a hidden text layer on the PDF.
    pub overlay_text: bool,
    /// Output path for the OCR'd PDF (if overlay_text is true).
    pub output_path: Option<String>,
    /// Language hints (e.g., "eng", "fra", "deu"). Defaults to "eng".
    pub language: String,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            pages: Vec::new(),
            backend: OcrBackend::Tesseract,
            overlay_text: true,
            output_path: None,
            language: "eng".to_string(),
        }
    }
}

/// Detection of whether a PDF is text-based or scanned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdfType {
    /// PDF contains embedded text and fonts (searchable).
    TextBased,
    /// PDF is primarily image-based (scanned document).
    Scanned,
    /// PDF is mixed (some pages text-based, some scanned).
    Mixed { text_pages: Vec<usize>, scanned_pages: Vec<usize> },
}

/// Analyze a PDF to determine if it's text-based or scanned.
///
/// Uses heuristics:
/// - Check for embedded fonts and text operators
/// - Render pages and detect image content
/// - Return per-page classification
pub fn detect_pdf_type(pdf_path: &PathBuf) -> Result<PdfType, String> {
    // TODO: Implement detection logic
    // 1. Load PDF with lopdf
    // 2. Check for text operators and fonts in each page's content stream
    // 3. For pages without text, mark as potentially scanned
    // 4. Return classification

    // Placeholder: assume text-based for now
    Ok(PdfType::TextBased)
}

/// Run OCR on a PDF using the specified backend.
pub fn run_ocr(pdf_path: &PathBuf, options: OcrOptions) -> Result<OcrResult, String> {
    let start = std::time::Instant::now();

    // Validate the PDF exists and is readable
    if !pdf_path.exists() {
        return Err(format!("PDF not found: {}", pdf_path.display()));
    }

    // Determine which pages to process
    let pages_to_process = if options.pages.is_empty() {
        // TODO: Get total page count from PDF
        vec![0] // placeholder
    } else {
        options.pages.clone()
    };

    // Route to the appropriate backend
    let results = match options.backend {
        OcrBackend::Tesseract => run_tesseract_ocr(pdf_path, &pages_to_process, &options)?,
        OcrBackend::GoogleCloudVision => run_google_vision_ocr(pdf_path, &pages_to_process, &options)?,
    };

    // If overlay_text is requested, overlay results onto output PDF
    if options.overlay_text {
        if let Some(output_path) = &options.output_path {
            overlay_ocr_text(pdf_path, output_path, &results)?;
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(OcrResult {
        pages_processed: results.len(),
        pages: results,
        backend: options.backend.as_str().to_string(),
        total_text: String::new(), // TODO: concatenate all page texts
        duration_ms,
    })
}

/// Run OCR using local Tesseract engine.
fn run_tesseract_ocr(
    pdf_path: &PathBuf,
    pages: &[usize],
    options: &OcrOptions,
) -> Result<Vec<OcrPageResult>, String> {
    // TODO: Implement Tesseract integration
    // 1. Render each page to an image
    // 2. Call Tesseract on each image
    // 3. Parse output and build OcrPageResult
    // 4. Return results

    Err("Tesseract OCR not yet implemented".to_string())
}

/// Run OCR using Google Cloud Vision API.
fn run_google_vision_ocr(
    pdf_path: &PathBuf,
    pages: &[usize],
    options: &OcrOptions,
) -> Result<Vec<OcrPageResult>, String> {
    // TODO: Implement Google Cloud Vision integration
    // 1. Get API key from keychain/settings
    // 2. Render each page to an image
    // 3. Call Cloud Vision API
    // 4. Parse response and build OcrPageResult
    // 5. Return results

    Err("Google Cloud Vision OCR not yet implemented".to_string())
}

/// Overlay OCR text as a hidden (searchable) text layer on a PDF.
///
/// This preserves the original PDF appearance while making it searchable.
/// The text is rendered in white on white (or transparent) so it's invisible
/// but selectable/searchable.
fn overlay_ocr_text(
    input_path: &PathBuf,
    output_path: &str,
    results: &[OcrPageResult],
) -> Result<(), String> {
    // TODO: Implement text layer overlay
    // 1. Load the original PDF
    // 2. For each page in results:
    //    a. Create a text operator for the extracted text
    //    b. Position it using the region bounding boxes
    //    c. Set text color to white/transparent
    //    d. Append to the page's content stream
    // 3. Save to output_path

    Err("Text overlay not yet implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_backend_as_str() {
        assert_eq!(OcrBackend::Tesseract.as_str(), "tesseract");
        assert_eq!(OcrBackend::GoogleCloudVision.as_str(), "google_cloud_vision");
    }

    #[test]
    fn test_ocr_options_default() {
        let opts = OcrOptions::default();
        assert_eq!(opts.backend, OcrBackend::Tesseract);
        assert!(opts.overlay_text);
        assert_eq!(opts.language, "eng");
    }
}
