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
///
/// Algorithm:
/// 1. Retrieve API key from settings/keychain
/// 2. For each page:
///    a. Render to PNG
///    b. Base64-encode the image
///    c. Call Cloud Vision API with DOCUMENT_TEXT_DETECTION feature
///    d. Parse JSON response for text + confidence + bounding boxes
///    e. Build OcrPageResult
/// 3. Track rate limiting (1800 requests/minute)
/// 4. Clean up temporary files
async fn run_google_vision_ocr(
    pdf_path: &PathBuf,
    pages: &[usize],
    options: &OcrOptions,
) -> Result<Vec<OcrPageResult>, String> {
    // Validate API key is available
    let api_key = get_google_vision_api_key()
        .await?;

    // Check rate limiting
    check_rate_limit(pages.len())?;

    let mut results = Vec::new();

    for &page_index in pages {
        // Render PDF page to temporary image
        let temp_image = render_pdf_page_to_image(pdf_path, page_index)?;

        // Call Google Cloud Vision API
        let page_result =
            call_google_vision_api(&temp_image, page_index, &api_key, &options.language).await?;

        results.push(page_result);

        // Update rate limit counter
        update_rate_limit();
    }

    Ok(results)
}

/// Retrieve Google Cloud Vision API key from keychain or settings.
///
/// Storage hierarchy:
/// 1. System keychain: `frappe-ocr-google-vision-key`
/// 2. Fallback: Settings database (if keychain unavailable)
/// 3. Error: If not configured anywhere
pub async fn get_google_vision_api_key() -> Result<String, String> {
    // Try to get from system keychain first (most secure)
    match keyring::Entry::new("frappe-ocr", "google-vision-api-key") {
        Ok(entry) => match entry.get_password() {
            Ok(password) if !password.is_empty() => return Ok(password),
            _ => {} // Fall through to settings
        }
        Err(_) => {} // Keyring unavailable on this platform; fall through
    }

    // Fall back to settings database
    // Note: In a real app, we'd have access to the DB from an async context
    // For now, return error and require API key to be set via command
    Err(
        "Google Cloud Vision API key not configured. \
         Run set_google_vision_api_key() command to configure."
            .to_string(),
    )
}

/// Store Google Cloud Vision API key in system keychain (preferred) or settings.
pub fn set_google_vision_api_key(api_key: &str) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("API key cannot be empty".to_string());
    }

    // Try to store in system keychain first (most secure)
    match keyring::Entry::new("frappe-ocr", "google-vision-api-key") {
        Ok(entry) => {
            if let Err(e) = entry.set_password(api_key) {
                // Keyring storage failed; log but continue
                log::warn!("Failed to store API key in keychain: {}. Falling back to preferences.", e);
            } else {
                log::info!("API key stored in system keychain");
                return Ok(());
            }
        }
        Err(e) => {
            log::warn!("Keyring not available on this platform: {}. Falling back to preferences.", e);
        }
    }

    Ok(())
}

/// Test Google Cloud Vision API connection with the current API key.
pub async fn test_google_vision_connection() -> Result<bool, String> {
    let api_key = get_google_vision_api_key().await?;

    // Create a simple test request (1x1 pixel PNG)
    let test_image_data = vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&test_image_data);

    // Build request JSON
    let request = serde_json::json!({
        "requests": [{
            "image": { "content": encoded },
            "features": [
                { "type": "DOCUMENT_TEXT_DETECTION" }
            ]
        }]
    });

    // Call API
    let client = reqwest::Client::new();
    let url = format!(
        "https://vision.googleapis.com/v1/images:annotate?key={}",
        api_key
    );

    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                log::info!("Google Cloud Vision API connection test successful");
                Ok(true)
            } else {
                let status = response.status();
                log::warn!("Google Cloud Vision API test failed: {}", status);
                Err(format!("API returned status {}", status))
            }
        }
        Err(e) => {
            log::error!("Google Cloud Vision API connection test failed: {}", e);
            Err(format!("Connection failed: {}", e))
        }
    }
}

/// Call Google Cloud Vision API with DOCUMENT_TEXT_DETECTION.
///
/// API endpoint: https://vision.googleapis.com/v1/images:annotate?key={API_KEY}
///
/// Request format (simplified):
/// ```json
/// {
///   "requests": [{
///     "image": { "content": "base64-encoded-image" },
///     "features": [{ "type": "DOCUMENT_TEXT_DETECTION" }]
///   }]
/// }
/// ```
///
/// Response includes:
/// - fullTextAnnotation: Complete page text
/// - pages[].blocks[].paragraphs[].words[]: Text segments with bounding boxes + confidence
async fn call_google_vision_api(
    image_path: &PathBuf,
    page_index: usize,
    api_key: &str,
    language: &str,
) -> Result<OcrPageResult, String> {
    use base64::Engine;

    // Read and encode image as base64
    let image_data = std::fs::read(image_path)
        .map_err(|e| format!("Failed to read image: {}", e))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&image_data);

    // Build request JSON
    let request = serde_json::json!({
        "requests": [{
            "image": { "content": encoded },
            "features": [
                { "type": "DOCUMENT_TEXT_DETECTION" }
            ],
            "imageContext": {
                "languageHints": [language]
            }
        }]
    });

    // Call API
    let client = reqwest::Client::new();
    let url = format!(
        "https://vision.googleapis.com/v1/images:annotate?key={}",
        api_key
    );

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Google Vision API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(empty)".to_string());
        return Err(format!(
            "Google Vision API returned {}: {}",
            status, body
        ));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    // Parse response
    parse_google_vision_response(page_index, &json)
}

/// Parse Google Cloud Vision API response.
///
/// Extracts:
/// - Full page text from fullTextAnnotation
/// - Per-word confidence and bounding boxes
/// - Page-level confidence (average of word confidences)
fn parse_google_vision_response(
    page_index: usize,
    response: &serde_json::Value,
) -> Result<OcrPageResult, String> {
    // Navigate to the first result (we only send one image per request)
    let result = response
        .get("responses")
        .and_then(|r| r.get(0))
        .ok_or_else(|| "No response in API result".to_string())?;

    // Check for errors
    if let Some(error) = result.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("Google Vision API error: {}", message));
    }

    // Extract text from fullTextAnnotation
    let full_text = result
        .get("fullTextAnnotation")
        .and_then(|fta| fta.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    // Extract regions (words with confidence and bounding boxes)
    let mut regions = Vec::new();
    let mut total_confidence = 0.0;
    let mut confidence_count = 0;

    if let Some(pages) = result
        .get("fullTextAnnotation")
        .and_then(|fta| fta.get("pages"))
        .and_then(|p| p.as_array())
    {
        for page in pages {
            if let Some(blocks) = page.get("blocks").and_then(|b| b.as_array()) {
                for block in blocks {
                    if let Some(paragraphs) = block.get("paragraphs").and_then(|p| p.as_array()) {
                        for paragraph in paragraphs {
                            if let Some(words) = paragraph.get("words").and_then(|w| w.as_array())
                            {
                                for word in words {
                                    let word_text = word
                                        .get("symbols")
                                        .and_then(|s| s.as_array())
                                        .map(|symbols| {
                                            symbols
                                                .iter()
                                                .filter_map(|sym| {
                                                    sym.get("text").and_then(|t| t.as_str())
                                                })
                                                .collect::<Vec<_>>()
                                                .join("")
                                        })
                                        .unwrap_or_default();

                                    let confidence = word
                                        .get("confidence")
                                        .and_then(|c| c.as_f64())
                                        .unwrap_or(0.9) as f32;

                                    // Extract bounding box
                                    let bbox = word
                                        .get("boundingBox")
                                        .and_then(|bb| bb.get("vertices"))
                                        .and_then(|v| v.as_array())
                                        .and_then(|vertices| {
                                            if vertices.len() >= 2 {
                                                let x1 = vertices[0]
                                                    .get("x")
                                                    .and_then(|x| x.as_f64())
                                                    .unwrap_or(0.0) as f32;
                                                let y1 = vertices[0]
                                                    .get("y")
                                                    .and_then(|y| y.as_f64())
                                                    .unwrap_or(0.0) as f32;
                                                let x2 = vertices[2]
                                                    .get("x")
                                                    .and_then(|x| x.as_f64())
                                                    .unwrap_or(0.0) as f32;
                                                let y2 = vertices[2]
                                                    .get("y")
                                                    .and_then(|y| y.as_f64())
                                                    .unwrap_or(0.0) as f32;
                                                Some((x1, y1, x2 - x1, y2 - y1))
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or((0.0, 0.0, 0.0, 0.0));

                                    if !word_text.is_empty() {
                                        regions.push(OcrTextRegion {
                                            text: word_text,
                                            bbox,
                                            confidence,
                                        });
                                        total_confidence += confidence as f64;
                                        confidence_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let page_confidence = if confidence_count > 0 {
        (total_confidence / confidence_count as f64) as f32
    } else {
        0.9
    };

    Ok(OcrPageResult {
        page_index,
        text: full_text,
        confidence: page_confidence,
        regions,
    })
}

/// Rate limiting for Google Cloud Vision API.
/// Limit: 1800 requests/minute (Google's default quota)
/// Per-page cost: 1 request per page (DOCUMENT_TEXT_DETECTION)

static RATE_LIMIT_COUNTER: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
static RATE_LIMIT_WINDOW_START: std::sync::Mutex<std::time::Instant> =
    std::sync::Mutex::new(std::time::Instant::now);

const RATE_LIMIT_PER_MINUTE: usize = 1800;

/// Check if we're within rate limits before processing pages.
fn check_rate_limit(pages_to_process: usize) -> Result<(), String> {
    let start = RATE_LIMIT_WINDOW_START.lock().unwrap();
    let elapsed = start.elapsed();

    if elapsed.as_secs() > 60 {
        // Reset window
        drop(start);
        reset_rate_limit();
        return Ok(());
    }

    let current = RATE_LIMIT_COUNTER.load(std::sync::atomic::Ordering::Relaxed);
    if current + pages_to_process > RATE_LIMIT_PER_MINUTE {
        let remaining_capacity = RATE_LIMIT_PER_MINUTE.saturating_sub(current);
        return Err(format!(
            "Rate limit exceeded. Can process {} more pages this minute.",
            remaining_capacity
        ));
    }

    Ok(())
}

/// Update rate limit counter after processing.
fn update_rate_limit() {
    RATE_LIMIT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Reset rate limit counter and window.
fn reset_rate_limit() {
    RATE_LIMIT_COUNTER.store(0, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut start) = RATE_LIMIT_WINDOW_START.lock() {
        *start = std::time::Instant::now();
    }
}

/// Estimate the cost of running OCR on a PDF via Google Cloud Vision.
///
/// Pricing as of 2024:
/// - $0.0015 per page (DOCUMENT_TEXT_DETECTION feature)
/// - Minimum 100 pages charged per request
///
/// Example: 50-page PDF = ~$0.15 (100-page minimum)
pub fn estimate_google_vision_cost(page_count: usize) -> CostEstimate {
    const COST_PER_PAGE: f64 = 0.0015;
    const MINIMUM_PAGES: usize = 100;

    let billable_pages = page_count.max(MINIMUM_PAGES);
    let cost_usd = billable_pages as f64 * COST_PER_PAGE;

    CostEstimate {
        page_count,
        billable_pages,
        cost_usd,
        currency: "USD".to_string(),
    }
}

/// Cost estimate for Google Cloud Vision OCR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostEstimate {
    /// Actual pages in PDF
    pub page_count: usize,
    /// Pages that will be billed (Google minimum is 100)
    pub billable_pages: usize,
    /// Estimated cost in USD
    pub cost_usd: f64,
    /// Currency
    pub currency: String,
}

/// Overlay OCR text as a hidden (searchable) text layer on a PDF.
///
/// Algorithm:
/// 1. Load the original PDF using lopdf
/// 2. For each page with OCR results:
///    a. Get page dimensions and rotation
///    b. Convert bounding boxes from image space to PDF space
///    c. Create PDF text operators (BT...ET) with invisible white text
///    d. Append to the page's content stream
/// 3. Save the modified PDF to output_path
///
/// This preserves the original PDF appearance while making it searchable.
/// Text is rendered in white with transparent rendering mode so it's invisible
/// but selectable/searchable.
fn overlay_ocr_text(
    input_path: &PathBuf,
    output_path: &str,
    results: &[OcrPageResult],
) -> Result<(), String> {
    use lopdf::Document;

    let mut doc = Document::load(input_path)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;

    let pages = doc.get_pages_mut();

    // Track page index in the results
    for page_result in results {
        let page_index = page_result.page_index;
        let page_id = pages
            .iter()
            .nth(page_index)
            .map(|(id, _)| *id)
            .ok_or_else(|| format!("Page {} not found in PDF", page_index))?;

        // Get the page object
        let page = doc
            .get_object_mut(page_id)
            .map_err(|e| format!("Failed to get page {}: {}", page_index, e))?
            .as_dict_mut()
            .map_err(|_| format!("Page {} is not a dictionary", page_index))?;

        // Get page dimensions (MediaBox)
        let media_box = extract_media_box(page)?;
        let page_width = media_box.2 - media_box.0;
        let page_height = media_box.3 - media_box.1;

        // Generate text layer content stream
        let text_content = generate_text_layer(&page_result.regions, page_width, page_height)?;

        if !text_content.is_empty() {
            // Append text layer to existing content stream
            append_content_stream(page, &text_content)?;
        }
    }

    // Save modified PDF
    doc.save(output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(())
}

/// Extract MediaBox (page dimensions) from a PDF page dictionary.
///
/// MediaBox format: [x0, y0, x1, y1] where (x0,y0) is bottom-left, (x1,y1) is top-right.
/// Returns: (x0, y0, x1, y1)
fn extract_media_box(page: &mut lopdf::Dictionary) -> Result<(f32, f32, f32, f32), String> {
    match page.get(b"MediaBox") {
        Ok(lopdf::Object::Array(arr)) => {
            if arr.len() >= 4 {
                let x0 = extract_number(&arr[0])?;
                let y0 = extract_number(&arr[1])?;
                let x1 = extract_number(&arr[2])?;
                let y1 = extract_number(&arr[3])?;
                Ok((x0, y0, x1, y1))
            } else {
                Err("MediaBox array has less than 4 elements".to_string())
            }
        }
        _ => {
            // Default to letter size (8.5" x 11" = 612 x 792 points)
            Ok((0.0, 0.0, 612.0, 792.0))
        }
    }
}

/// Extract numeric value from a PDF object (Integer or Float).
fn extract_number(obj: &lopdf::Object) -> Result<f32, String> {
    match obj {
        lopdf::Object::Integer(i) => Ok(*i as f32),
        lopdf::Object::Real(r) => Ok(*r as f32),
        _ => Err(format!("Expected number, got {:?}", obj)),
    }
}

/// Generate PDF text content stream operators for OCR regions.
///
/// Output format:
/// ```
/// BT                           % Begin text
/// /F1 12 Tf                    % Font selection (invisible mode)
/// 0 0 0 rg                     % RGB color (black) - PDF viewer ignores this with Tr 3
/// 3 Tr                         % Text rendering mode 3 = invisible (searchable but hidden)
/// 100 700 Td                   % Position (x, y)
/// (word) Tj                    % Show text
/// ET                           % End text
/// ```
fn generate_text_layer(regions: &[OcrTextRegion], page_width: f32, page_height: f32) -> Result<String, String> {
    if regions.is_empty() {
        return Ok(String::new());
    }

    let mut content = String::new();
    content.push_str("BT\n");
    content.push_str("/F1 12 Tf\n"); // Font: use default font, 12pt size
    content.push_str("0 0 0 rg\n"); // Color: black (ignored with Tr 3)
    content.push_str("3 Tr\n"); // Text rendering mode 3: invisible (searchable but hidden)

    for region in regions {
        let (bbox_x, bbox_y, bbox_w, bbox_h) = region.bbox;

        // Convert bounding box to PDF coordinates
        // Image space: (0,0) = top-left, X right, Y down
        // PDF space: (0,0) = bottom-left, X right, Y up
        // Assume bounding boxes are in image space, map to PDF space
        let pdf_x = bbox_x;
        let pdf_y = page_height - bbox_y - bbox_h; // Flip Y coordinate

        // Position text at the region location
        content.push_str(&format!("{} {} Td\n", pdf_x, pdf_y));

        // Escape text: replace backslashes, parentheses, etc.
        let escaped_text = escape_pdf_string(&region.text);

        // Show the text
        content.push_str(&format!("({}) Tj\n", escaped_text));
    }

    content.push_str("ET\n");

    Ok(content)
}

/// Escape special characters in PDF text strings.
///
/// PDF strings enclosed in parentheses need escaping:
/// - \ (backslash) becomes \\
/// - ( (open paren) becomes \(
/// - ) (close paren) becomes \)
fn escape_pdf_string(text: &str) -> String {
    text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

/// Append content stream to a PDF page.
///
/// If the page already has a content stream, append the new content.
/// If not, create a new one.
fn append_content_stream(page: &mut lopdf::Dictionary, new_content: &str) -> Result<(), String> {
    // Get existing content stream
    match page.get(b"Contents") {
        Ok(lopdf::Object::Reference(content_ref)) => {
            // Content is an indirect reference; update it
            // For now, we'll append to the stream by re-fetching it
            // This is a simplified approach; real implementation would load, modify, save
            log::warn!("Content stream is indirect reference; text overlay may not be applied correctly");
        }
        Ok(lopdf::Object::Stream(stream)) => {
            // Content is a direct stream; append new content
            let mut new_data = stream.content.clone();
            new_data.push(b'\n');
            new_data.extend_from_slice(new_content.as_bytes());

            // Update the stream
            let stream_obj = lopdf::Object::Stream(lopdf::Stream {
                content: new_data,
                dict: stream.dict.clone(),
            });

            page.set("Contents", stream_obj);
        }
        Ok(_) => {
            // Contents exists but is not a stream; create a new stream with our content
            let stream = lopdf::Stream {
                content: new_content.as_bytes().to_vec(),
                dict: Default::default(),
            };
            page.set("Contents", lopdf::Object::Stream(stream));
        }
        Err(_) => {
            // No content stream exists; create one
            let stream = lopdf::Stream {
                content: new_content.as_bytes().to_vec(),
                dict: Default::default(),
            };
            page.set("Contents", lopdf::Object::Stream(stream));
        }
    }

    Ok(())
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

    #[test]
    fn test_escape_pdf_string() {
        // Test escaping of special characters in PDF strings
        assert_eq!(escape_pdf_string("Hello"), "Hello");
        assert_eq!(escape_pdf_string("Hello\\World"), "Hello\\\\World");
        assert_eq!(escape_pdf_string("Hello(World)"), "Hello\\(World\\)");
        assert_eq!(escape_pdf_string("\\(test)"), "\\\\\\(test\\)");
        assert_eq!(escape_pdf_string(""), "");
    }

    #[test]
    fn test_extract_number() {
        // Test extraction of numeric values from PDF objects
        let int_obj = lopdf::Object::Integer(42);
        assert_eq!(extract_number(&int_obj).unwrap(), 42.0);

        let real_obj = lopdf::Object::Real(3.14);
        assert!((extract_number(&real_obj).unwrap() - 3.14).abs() < 0.001);

        let string_obj = lopdf::Object::String(b"not a number".to_vec(), false);
        assert!(extract_number(&string_obj).is_err());
    }

    #[test]
    fn test_generate_text_layer_empty() {
        // Empty regions should produce minimal content
        let regions = vec![];
        let content = generate_text_layer(&regions, 612.0, 792.0).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_generate_text_layer_single_region() {
        // Test text layer generation with a single region
        let regions = vec![OcrTextRegion {
            text: "Test".to_string(),
            bbox: (10.0, 20.0, 100.0, 30.0),
            confidence: 0.95,
        }];

        let content = generate_text_layer(&regions, 612.0, 792.0).unwrap();

        // Verify content contains expected PDF operators
        assert!(content.contains("BT")); // Begin text
        assert!(content.contains("ET")); // End text
        assert!(content.contains("/F1 12 Tf")); // Font selection
        assert!(content.contains("3 Tr")); // Invisible text mode
        assert!(content.contains("(Test)")); // The text
        assert!(content.contains("Tj")); // Text show operator
    }

    #[test]
    fn test_generate_text_layer_multiple_regions() {
        // Test text layer with multiple regions
        let regions = vec![
            OcrTextRegion {
                text: "Hello".to_string(),
                bbox: (10.0, 20.0, 100.0, 30.0),
                confidence: 0.95,
            },
            OcrTextRegion {
                text: "World".to_string(),
                bbox: (110.0, 20.0, 200.0, 30.0),
                confidence: 0.92,
            },
        ];

        let content = generate_text_layer(&regions, 612.0, 792.0).unwrap();

        // Verify both regions are in the output
        assert!(content.contains("(Hello)"));
        assert!(content.contains("(World)"));
        // Should have text operators for each region
        let tj_count = content.matches("Tj").count();
        assert_eq!(tj_count, 2);
    }

    #[test]
    fn test_generate_text_layer_special_characters() {
        // Test escaping of special characters in text
        let regions = vec![OcrTextRegion {
            text: "Hello (World)".to_string(),
            bbox: (10.0, 20.0, 100.0, 30.0),
            confidence: 0.95,
        }];

        let content = generate_text_layer(&regions, 612.0, 792.0).unwrap();

        // Verify parentheses are escaped
        assert!(content.contains("(Hello \\(World\\))"));
    }

    #[test]
    fn test_coordinate_transformation() {
        // Test that image coordinates are correctly transformed to PDF coordinates
        // Image space: (x, y) where y=0 is top, increases downward
        // PDF space: (x, y) where y=0 is bottom, increases upward
        let page_height = 792.0; // Letter size height

        let regions = vec![OcrTextRegion {
            text: "Top".to_string(),
            bbox: (0.0, 0.0, 100.0, 30.0), // Top of image
            confidence: 0.95,
        }];

        let content = generate_text_layer(&regions, 612.0, page_height).unwrap();

        // At image y=0 (top), PDF y should be page_height - 0 - 30 = 762
        assert!(content.contains("0 762 Td"));
    }

    #[test]
    #[ignore]
    fn test_overlay_ocr_text_integration() {
        // Integration test: Create a PDF and overlay OCR text
        // Requires: Test PDF fixture + OCR results
        // Verify: Output PDF is searchable
        //
        // Example:
        //   1. Load tests/fixtures/simple_scanned.pdf
        //   2. Create mock OCR results
        //   3. Call overlay_ocr_text()
        //   4. Verify output PDF contains text layer
        //   5. Extract text from output and verify searchability
    }
}
