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
/// - Check for embedded fonts and text operators via lopdf
/// - Classify each page independently
/// - Return overall classification (TextBased, Scanned, or Mixed)
pub fn detect_pdf_type(pdf_path: &PathBuf) -> Result<PdfType, String> {
    use lopdf::Document;

    let doc = Document::load(pdf_path)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;

    let page_count = doc.get_pages().len();
    if page_count == 0 {
        return Err("PDF has no pages".to_string());
    }

    let mut text_pages = Vec::new();
    let mut scanned_pages = Vec::new();

    // Check each page for text content
    for (page_index, (page_id, _)) in doc.get_pages().iter().enumerate() {
        let has_text = has_page_text(&doc, *page_id)?;

        if has_text {
            text_pages.push(page_index);
        } else {
            scanned_pages.push(page_index);
        }
    }

    // Classify the PDF based on page breakdown
    if scanned_pages.is_empty() {
        Ok(PdfType::TextBased)
    } else if text_pages.is_empty() {
        Ok(PdfType::Scanned)
    } else {
        Ok(PdfType::Mixed {
            text_pages,
            scanned_pages,
        })
    }
}

/// Check if a PDF page contains text operators (indicating text content).
///
/// Heuristic: Look for text operators in the content stream:
/// - BT (Begin Text)
/// - Tj / TJ (Show Text)
/// - Td / TD / T* (Text positioning)
/// If found, the page likely has embedded text.
fn has_page_text(doc: &lopdf::Document, page_id: (u32, u16)) -> Result<bool, String> {
    let page = doc
        .get_object_mut(page_id)
        .map_err(|e| format!("Failed to get page object: {}", e))?
        .as_dict_mut()
        .map_err(|_| "Page is not a dictionary".to_string())?;

    // Get content stream (may be direct or indirect reference)
    let content = match page.get(b"Contents") {
        Ok(lopdf::Object::Reference(content_ref)) => {
            let content_obj = doc
                .get_object(*content_ref)
                .map_err(|e| format!("Failed to get content stream: {}", e))?;
            content_obj.as_stream().ok()
        }
        Ok(lopdf::Object::Stream(stream)) => Some(stream),
        _ => None,
    };

    if let Some(stream) = content {
        let content_data = String::from_utf8_lossy(&stream.content);

        // Check for text operators
        let has_text_ops = content_data.contains(" BT ") || // Begin text
            content_data.contains(" Tj ") ||  // Show text
            content_data.contains(" TJ ") ||  // Show text with positioning
            content_data.contains(" Td ") ||  // Text matrix
            content_data.contains(" TD ") ||  // Text matrix
            content_data.contains(" T* ");     // Next line

        Ok(has_text_ops)
    } else {
        // No content stream = likely an image-only page
        Ok(false)
    }
}

/// Get the total number of pages in a PDF.
pub fn get_page_count(pdf_path: &PathBuf) -> Result<usize, String> {
    use lopdf::Document;

    let doc = Document::load(pdf_path)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;

    Ok(doc.get_pages().len())
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
        // If no pages specified, process all pages
        let total_pages = get_page_count(pdf_path)?;
        (0..total_pages).collect()
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

    // Concatenate all page texts
    let total_text = results
        .iter()
        .map(|page| page.text.clone())
        .collect::<Vec<_>>()
        .join("\n---PAGE BREAK---\n");

    Ok(OcrResult {
        pages_processed: results.len(),
        pages: results,
        backend: options.backend.as_str().to_string(),
        total_text,
        duration_ms,
    })
}

/// Run OCR using local Tesseract engine.
///
/// Algorithm:
/// 1. Check if tesseract binary is available
/// 2. For each page:
///    a. Render to PNG at 300 DPI
///    b. Invoke tesseract with hOCR output
///    c. Parse hOCR XML for text + bounding boxes
///    d. Build OcrPageResult with confidence scores
/// 3. Clean up temporary image files
fn run_tesseract_ocr(
    pdf_path: &PathBuf,
    pages: &[usize],
    options: &OcrOptions,
) -> Result<Vec<OcrPageResult>, String> {
    // Check if tesseract is available
    check_tesseract_available()?;

    let mut results = Vec::new();

    for &page_index in pages {
        // Render PDF page to temporary image
        let temp_image = render_pdf_page_to_image(pdf_path, page_index)?;

        // Run tesseract on the image
        let tesseract_text = run_tesseract_command(&temp_image, &options.language)?;

        // Parse tesseract output into structured result
        let page_result = parse_tesseract_output(page_index, &tesseract_text)?;

        results.push(page_result);
    }

    Ok(results)
}

/// Check if tesseract binary is available on the system PATH.
pub fn check_tesseract_available() -> Result<(), String> {
    match std::process::Command::new("tesseract")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => Err("tesseract command failed".to_string()),
        Err(_) => Err(
            "tesseract not found. Install from: https://github.com/UB-Mannheim/tesseract/wiki"
                .to_string(),
        ),
    }
}

/// Render a single PDF page to a temporary PNG image at 300 DPI.
fn render_pdf_page_to_image(pdf_path: &PathBuf, page_index: usize) -> Result<PathBuf, String> {
    use pdfium_render::prelude::*;

    // Load PDF
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::bind_to_system_library())
            .or_else(|_| Pdfium::bind_to_library(Pdfium::bind_to_builtin_library()))
            .map_err(|e| format!("Failed to initialize PDFium: {:?}", e))?,
    );

    let document = pdfium
        .load_pdf_from_file(&pdf_path, None)
        .map_err(|e| format!("Failed to load PDF: {:?}", e))?;

    // Get the specific page
    let page = document
        .pages()
        .get(page_index as u32)
        .ok_or_else(|| format!("Page {} not found", page_index))?;

    // Render at 300 DPI for OCR (standard for text recognition)
    let dpi = 300.0;
    let scale_factor = dpi / 72.0; // PDF uses 72 DPI as default

    let render_config = PdfRenderConfig::new()
        .set_maximum_width((page.width().value * scale_factor) as i32)
        .set_maximum_height((page.height().value * scale_factor) as i32);

    let bitmap = page
        .render_with_config(&render_config)
        .map_err(|e| format!("Failed to render page: {:?}", e))?
        .as_image();

    // Save to temporary file
    let temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let temp_path = temp_file.path().with_extension("png");
    bitmap
        .save(&temp_path)
        .map_err(|e| format!("Failed to save rendered image: {}", e))?;

    Ok(temp_path)
}

/// Invoke tesseract on an image and get text output.
///
/// Tesseract is invoked with:
/// - Input: image file path
/// - Output: text file path (tesseract adds .txt extension)
/// - Language: specified in options
/// - Config: quiet mode (minimal output)
fn run_tesseract_command(image_path: &PathBuf, language: &str) -> Result<String, String> {
    use std::process::Command;

    // Remove extension from image path for tesseract output
    let output_base = image_path.with_extension("");

    // Run: tesseract input.png output -l eng
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg(&output_base)
        .arg("-l")
        .arg(language)
        .arg("quiet") // Suppress progress messages
        .output()
        .map_err(|e| format!("Failed to run tesseract: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tesseract failed: {}", stderr));
    }

    // Read the output text file
    let output_file = output_base.with_extension("txt");
    let text = std::fs::read_to_string(&output_file)
        .map_err(|e| format!("Failed to read tesseract output: {}", e))?;

    // Clean up output file
    let _ = std::fs::remove_file(&output_file);

    Ok(text)
}

/// Parse tesseract text output into structured OcrPageResult.
///
/// Currently provides:
/// - Full page text
/// - Default confidence (90% for now; enhanced parsing in Phase 2b)
/// - Empty regions (will parse hOCR for bounding boxes in Phase 2b)
fn parse_tesseract_output(page_index: usize, text: &str) -> Result<OcrPageResult, String> {
    if text.trim().is_empty() {
        return Ok(OcrPageResult {
            page_index,
            text: String::new(),
            confidence: 0.0,
            regions: Vec::new(),
        });
    }

    Ok(OcrPageResult {
        page_index,
        text: text.trim().to_string(),
        // TODO: Phase 2b: Parse confidence from hOCR output
        confidence: 0.9,
        // TODO: Phase 2b: Extract bounding boxes from hOCR
        regions: Vec::new(),
    })
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

    #[test]
    fn test_check_tesseract_available() {
        // This test will pass/fail depending on whether tesseract is installed
        // In CI, we should skip if tesseract is not available
        if which::which("tesseract").is_ok() {
            assert!(check_tesseract_available().is_ok());
        } else {
            // Tesseract not installed; that's okay for unit tests
            // Integration tests can be skipped with #[ignore]
        }
    }

    #[test]
    fn test_ocr_page_result_construction() {
        let region = OcrTextRegion {
            text: "Hello World".to_string(),
            bbox: (10.0, 20.0, 100.0, 30.0),
            confidence: 0.95,
        };

        let result = OcrPageResult {
            page_index: 0,
            text: "Hello World".to_string(),
            confidence: 0.95,
            regions: vec![region],
        };

        assert_eq!(result.page_index, 0);
        assert_eq!(result.text, "Hello World");
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.regions.len(), 1);
    }

    #[test]
    fn test_parse_tesseract_output_empty() {
        let result = parse_tesseract_output(0, "").unwrap();
        assert_eq!(result.page_index, 0);
        assert_eq!(result.text, "");
        assert_eq!(result.confidence, 0.0);
        assert_eq!(result.regions.len(), 0);
    }

    #[test]
    fn test_parse_tesseract_output_with_text() {
        let text = "Hello World\nThis is OCR output";
        let result = parse_tesseract_output(1, text).unwrap();
        assert_eq!(result.page_index, 1);
        assert_eq!(result.text, "Hello World\nThis is OCR output");
        assert!(result.confidence > 0.0);
    }

    // Integration tests (require tesseract and PDF fixtures)

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn test_tesseract_ocr_integration() {
        // This test requires:
        // 1. Tesseract to be installed
        // 2. A scanned PDF fixture at tests/fixtures/simple_scanned.pdf
        //
        // Example usage:
        //   cargo test test_tesseract_ocr_integration -- --ignored --nocapture
        //
        // Fixture: A simple scanned image converted to PDF with known text
    }

    #[test]
    #[ignore]
    fn test_detect_pdf_type_text_based() {
        // Requires: tests/fixtures/text_document.pdf (PDF with embedded text)
    }

    #[test]
    #[ignore]
    fn test_detect_pdf_type_scanned() {
        // Requires: tests/fixtures/simple_scanned.pdf (image-only PDF)
    }

    #[test]
    #[ignore]
    fn test_detect_pdf_type_mixed() {
        // Requires: tests/fixtures/mixed_document.pdf (some pages text, some scanned)
    }
}
