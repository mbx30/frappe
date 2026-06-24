# Issue #229: Optical Character Recognition (OCR) Implementation Roadmap

**Status:** Phase 1 Scaffolding Complete  
**Created:** 2026-06-24  
**Target Completion:** Phase 3 (2026-Q3)

## Overview

Implement OCR to convert scanned PDFs and images into searchable and editable text. This feature enables professional PDF editing workflows and accessibility compliance.

## Phase 1: Scaffolding ✅ Complete

### What's Been Built

1. **OCR Module** (`src-tauri/src/pdf/ocr.rs`)
   - `PdfType` enum: TextBased, Scanned, Mixed
   - `OcrBackend` enum: Tesseract, GoogleCloudVision
   - `OcrOptions` struct: Configuration for OCR runs
   - `OcrResult` struct: Full OCR output with confidence scores
   - `OcrPageResult` and `OcrTextRegion`: Per-page and per-region results
   - Placeholder implementations (stubs return "not implemented")

2. **Tauri Commands** (`src-tauri/src/commands.rs`)
   - `detect_pdf_type(path: String) -> Result<PdfType, String>`
     - Analyzes PDF to determine if it's text-based or scanned
     - Returns classification per page for mixed PDFs
   
   - `run_ocr(path: String, options: OcrOptions) -> Result<OcrResult, String>`
     - Runs OCR with backend selection
     - Supports selective page processing
     - Optional text layer overlay
     - Language hints and confidence reporting

3. **Type Definitions**
   - All types implement `Serialize`/`Deserialize` for Tauri IPC
   - Proper error handling with `Result` types
   - Support for optional output path and language hints

### Security Considerations (Phase 1)

- ✅ Path validation using `security::validate_read_path()` on input PDFs
- ✅ Path validation using `security::validate_write_path()` on output paths
- ✅ API key handling prepared for keychain storage (Phase 2)
- ⚠️ Todo: Rate limiting for Cloud Vision API (Phase 2)

## Phase 2: Backend Integration (Local Tesseract)

### Tesseract Implementation

1. **Tesseract Detection**
   - Check if `tesseract` binary is available on PATH
   - Fall back gracefully if not installed
   - Expose binary location as a setting

2. **Page Rendering**
   - Use `pdfium-render` to render each page to PNG/JPEG
   - Handle DPI settings (recommend 300 DPI for OCR accuracy)
   - Cache rendered images during batch processing

3. **OCR Execution**
   - Invoke tesseract CLI: `tesseract image.png output -l {language} pdf`
   - Parse tesseract stdout for text + confidence
   - Extract hOCR bounding box data
   - Handle errors (bad PDF, tesseract not found, etc.)

4. **Text Extraction**
   - Parse tesseract output to build `OcrPageResult`
   - Map bounding boxes to PDF coordinates
   - Collect confidence scores per word/region

### Commands to Implement

```rust
#[tauri::command]
pub fn check_tesseract_available() -> Result<TesseractInfo, String>

#[tauri::command]
pub fn set_tesseract_path(path: String) -> Result<(), String>
```

### Settings

Add to preferences:
```json
{
  "ocr": {
    "backend": "tesseract",
    "tesseract_path": "/usr/bin/tesseract",
    "ocr_dpi": 300,
    "ocr_language": "eng"
  }
}
```

## Phase 3: Backend Integration (Google Cloud Vision API)

### Google Cloud Vision Implementation

1. **API Key Management**
   - Store API key in system keychain
   - Validate key format before use
   - Rate limit: 1800 requests/minute (Google default)

2. **Image Upload**
   - Render PDF pages to images (PNG, JPEG)
   - Upload to Cloud Vision API
   - Handle large files (split multi-page batches)

3. **Response Parsing**
   - Parse JSON response with text annotations
   - Extract bounding boxes
   - Map confidence scores
   - Build `OcrPageResult`

4. **Error Handling**
   - Network errors (retry with backoff)
   - API quota exceeded (show helpful error)
   - Invalid credentials (prompt for re-auth)
   - Large PDF (warn about costs/time)

### Commands to Implement

```rust
#[tauri::command]
pub async fn set_google_vision_api_key(key: String) -> Result<(), String>

#[tauri::command]
pub async fn test_google_vision_connection() -> Result<bool, String>

#[tauri::command]
pub fn estimate_google_vision_cost(pdf_path: String) -> Result<CostEstimate, String>
```

### Settings

Add to preferences:
```json
{
  "ocr": {
    "backend": "google_cloud_vision",
    "google_vision_api_key": "***", // stored in keychain
    "ocr_dpi": 300,
    "ocr_language": "eng",
    "enable_cost_warnings": true
  }
}
```

## Phase 4: Text Layer Overlay

### PDF Text Layer Implementation

The core feature: make scanned PDFs searchable without changing appearance.

1. **Render-to-Text-Layer Algorithm**
   - For each page in OCR results:
     - Create a PDF content stream operator for text
     - Position text using `OcrTextRegion` bounding boxes
     - Set text color to white (or transparent) so it's invisible
     - Append to the page's content stream
   - Write the modified PDF to output_path

2. **Accuracy Improvements**
   - Calibrate bounding boxes to account for PDF rotation
   - Handle page scaling (different media boxes)
   - Support custom text positioning (word-level vs line-level)

3. **Performance**
   - Process pages in parallel (spawn_blocking for each page)
   - Cache parsed PDF structure
   - Stream large files to disk rather than loading in memory

### Commands Already Support This

```rust
run_ocr(..., OcrOptions {
    overlay_text: true,
    output_path: Some("/path/to/output.pdf"),
    ...
})
```

## Phase 5: UI Integration

### Frontend Components

1. **OCR Panel**
   - Button: "Run OCR"
   - Dropdown: Backend selection (Tesseract / Google Cloud Vision)
   - Checkbox: Overlay text layer
   - Dropdown: Language
   - Checkbox: Selected pages only
   - Progress bar during OCR

2. **Text Detection Display**
   - Show PDF type (Text-Based / Scanned / Mixed)
   - Highlight scanned pages in page browser
   - Suggest OCR for scanned documents

3. **Results Display**
   - Extracted text preview
   - Per-page confidence scores
   - Time taken
   - Option to save OCR'd PDF

### Backend Commands Support This

All UI needs are already available:
- `detect_pdf_type()` — determine if OCR is needed
- `run_ocr()` — execute OCR with options
- Both support cancellation via response errors

## Phase 6: Advanced Features (Future)

1. **Batch OCR**
   - Process multiple PDFs in queue
   - Progress reporting per file
   - Automatic retry on errors

2. **OCR Confidence Filtering**
   - Re-run OCR on low-confidence regions
   - Use multiple passes (coarse → fine)
   - Human review for <90% confidence

3. **Language Detection**
   - Auto-detect page language
   - Multi-language PDFs (different languages per page)
   - Fallback to English if detection fails

4. **Export Options**
   - Export OCR results to searchable PDF
   - Export text layer to separate file
   - Export confidence metadata (JSON/CSV)

## Testing Strategy

### Unit Tests (Phase 1–2)

```rust
#[test]
fn test_ocr_options_default()

#[test]
fn test_pdf_type_detection_stub()

#[test]
fn test_tesseract_command_generation()
```

### Integration Tests (Phase 2–3)

```rust
#[test]
fn test_tesseract_ocr_on_scanned_pdf()

#[test]
fn test_google_vision_ocr_on_scanned_pdf()

#[test]
fn test_ocr_result_parsing()
```

### End-to-End Tests (Phase 4–5)

```rust
#[test]
fn test_ocr_overlay_on_scanned_pdf()

#[test]
fn test_overlaid_pdf_is_searchable()
```

### Test Fixtures

Required PDF fixtures in `tests/fixtures/`:
- `simple_scanned.pdf` — Single-page scanned document (English)
- `multi_page_scanned.pdf` — Multi-page scanned document
- `mixed_text_and_scanned.pdf` — Hybrid PDF (text + scanned pages)
- `low_quality_scan.pdf` — Poor quality scan (test confidence scores)

## Dependencies (To Add)

### Phase 2: Tesseract
```toml
[dependencies]
tempfile = "3"  # For temporary image files during OCR
```

No crate dependency for Tesseract; we'll invoke the binary via `std::process::Command`.

### Phase 3: Google Cloud Vision
```toml
[dependencies]
google-cloudvision1 = "5"  # Google Cloud Vision API client
# Or use reqwest directly for more control
```

### Already Available
- `image = "0.25"` — PDF page rendering
- `pdfium-render = "0.9"` — PDF manipulation
- `serde_json = "1"` — JSON parsing
- `reqwest = "0.12"` — HTTP client for API calls

## Acceptance Criteria

### Phase 1 ✅ Complete
- [x] OCR module with backend abstractions
- [x] Tauri commands (detect_pdf_type, run_ocr)
- [x] Type definitions (OcrOptions, OcrResult, etc.)
- [x] Path validation using security module
- [x] Placeholder implementations (stubs)

### Phase 2
- [ ] Tesseract backend implementation
- [ ] Page rendering to images
- [ ] Tesseract invocation and output parsing
- [ ] Settings for Tesseract binary path
- [ ] Error handling and fallbacks
- [ ] Tests with scanned PDF fixture

### Phase 3
- [ ] Google Cloud Vision API integration
- [ ] API key storage in keychain
- [ ] Rate limiting (1800 req/min)
- [ ] Cost estimation command
- [ ] Settings for API key + language
- [ ] Tests with Cloud Vision API mock

### Phase 4
- [ ] PDF text layer overlay logic
- [ ] Bounding box calibration
- [ ] Invisible white text positioning
- [ ] Output PDF generation
- [ ] Tests verifying searchability

### Phase 5
- [ ] Frontend OCR panel UI
- [ ] PDF type display
- [ ] Results preview
- [ ] Backend selection UI
- [ ] UI integration tests

## Timeline

| Phase | Task | Duration | Start | End |
|-------|------|----------|-------|-----|
| 1 | Scaffolding | 4h | 2026-06-24 | ✅ 2026-06-24 |
| 2 | Tesseract | 16h | 2026-06-24 | 2026-Q3 |
| 3 | Google Vision | 12h | 2026-Q3 | 2026-Q3 |
| 4 | Text Overlay | 8h | 2026-Q3 | 2026-Q3 |
| 5 | UI Integration | 12h | 2026-Q3 | 2026-Q3 |
| 6 | Advanced Features | TBD | 2026-Q4 | 2026-Q4+ |

**Total Phase 1–5:** ~52 hours

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Tesseract binary not found | Graceful fallback; helpful error message; settings to override path |
| Google Vision quota exceeded | Rate limiting; cost estimation before processing; queue-based processing |
| PDF rendering performance | Parallel page processing; cache rendered images; benchmark on large PDFs |
| OCR inaccuracy on poor scans | Per-page confidence; manual review UI; re-run with different settings |
| Overlay bounding box misalignment | Calibration tests; human verification; font metric adjustment |

---

## References

- [Tesseract OCR Documentation](https://github.com/UB-Mannheim/tesseract/wiki)
- [Google Cloud Vision API](https://cloud.google.com/vision/docs/ocr)
- [PDF Text Layer (Hidden Text) Specs](https://www.adobe.io/content/dam/udp/assets/open/pdf/spec/PDF32000_2008.pdf) (Section 5.3)
- [WCAG 2.1 Guideline 1.4.5 - Images of Text](https://www.w3.org/WAI/WCAG21/Understanding/images-of-text)
