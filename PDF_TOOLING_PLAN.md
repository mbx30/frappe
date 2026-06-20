# PDF Tooling Daily Implementation Plan

**Assumption:** 1 developer, 5 days/week, full-time. Each day = one focused work session.
**Total:** 270 days across 6 phases (~54 weeks / 13 months).

---

## Phase 1 — Preflight Foundation
*Issues: #21, #22, #24, #27, #28, #29, #30 | 50 days — Days 1-26 completed*

---

### ✓ Day 1 — Cargo setup + PDFium binary wiring
**Phase 1 | Week 1**
- Add `pdfium-render = { version = "0.9", features = ["sync"] }` and `lopdf = "0.41"` to `src-tauri/Cargo.toml`
- Download pre-built PDFium binaries: `pdfium.dll` (Windows x64) and `libpdfium.dylib` (macOS arm64) from the pdfium-render GitHub releases
- Add binaries to `src-tauri/resources/` and reference them in `tauri.conf.json` under `bundle.resources`
- Run `cargo check` — fix any dependency conflicts
✅ Done when: `cargo check` passes with pdfium-render and lopdf in the dependency tree

? ### Day 2 — PDFium initialization module
**Phase 1 | Week 1**
- Create `src-tauri/src/pdf/mod.rs` — new module for all PDF logic
- Write `PdfEngine` struct wrapping `pdfium_render::PdfiumRenderWasmConfig` or the native init
- Write `PdfEngine::init() -> Result<Self, String>` that loads the bundled PDFium binary via `Pdfium::bind_to_library(path)`
- Add `pdf` module to `lib.rs`, initialize `PdfEngine` once at startup via `manage()`
- Write a basic smoke test: open a hardcoded PDF path, assert page count > 0
✅ Done when: `cargo check` passes; manual test shows PDFium loads without panic

? ### Day 3 — `PdfSummary` struct + `open_pdf` command
**Phase 1 | Week 1**
- Add to `models.rs`:
  ```rust
  pub struct PdfSummary {
      pub id: i64,
      pub file_path: String,
      pub file_name: String,
      pub page_count: usize,
      pub pdf_version: String,
      pub file_size_bytes: u64,
      pub title: String,
      pub creator: String,
      pub producer: String,
      pub creation_date: String,
      pub is_encrypted: bool,
  }
  ```
- Write `commands::open_pdf(path: String) -> Result<PdfSummary, String>`:
  - Load via PDFium
  - Extract page count, version string (`%PDF-x.x` header)
  - Read Info dictionary (title, creator, producer, creation date) via lopdf
  - Check for `Encrypt` key in trailer via lopdf
  - Get file size via `std::fs::metadata`
- Register in `lib.rs`
✅ Done when: calling `open_pdf` from Tauri devtools returns a populated `PdfSummary`

? ### Day 4 — `pdf_jobs` table + persistence
**Phase 1 | Week 1**
- Add `pdf_jobs` table to `db.rs` inside `execute_batch`:
  ```sql
  CREATE TABLE IF NOT EXISTS pdf_jobs (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    page_count INTEGER NOT NULL,
    pdf_version TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    creator TEXT NOT NULL DEFAULT '',
    producer TEXT NOT NULL DEFAULT '',
    is_encrypted INTEGER NOT NULL DEFAULT 0,
    opened_at TEXT NOT NULL
  )
  ```
- Write `db::save_pdf_job(summary: &PdfSummary) -> Result<i64>`
- Write `db::list_pdf_jobs() -> Result<Vec<PdfSummary>>`
- Write `db::delete_pdf_job(id: i64) -> Result<()>`
- Add `list_pdf_jobs` and `delete_pdf_job` Tauri commands
✅ Done when: opening a PDF via `open_pdf` persists a row; `list_pdf_jobs` returns it

? ### Day 5 — PDFView skeleton in frontend
**Phase 1 | Week 1**
- Add `'pdf'` to `Section` type in `ManagementView.tsx`
- Add `{ id: 'pdf', label: 'PDF Tools', icon: '📄' }` to `NAV_ITEMS`
- Create `src/components/PDFView.tsx`:
  - File picker button using `@tauri-apps/plugin-dialog` with PDF filter (`{ name: 'PDF', extensions: ['pdf'] }`)
  - On file selected: call `open_pdf`, display `PdfSummary` card (file name, page count, PDF version, creator, encrypted badge)
  - Sidebar list of recent PDF jobs from `list_pdf_jobs`
- Create `src/components/PDFView.css`
✅ Done when: clicking "PDF Tools" in sidebar shows file picker; opening a PDF shows its metadata

? ### Day 6 — Page thumbnail strip
**Phase 1 | Week 1**
- Add `render_page_thumbnail(path: String, page_index: usize, width_px: u32) -> Result<String, String>` command:
  - Load PDF via PDFium
  - Render page to bitmap at `width_px` (default 120px)
  - Write bitmap to temp file via `std::env::temp_dir()`, return file path
- Frontend: `ThumbnailStrip` component — calls `render_page_thumbnail` for each page, displays as `<img>` row
- Add click-to-navigate: clicking thumbnail scrolls main view to that page
- Limit initial render to first 20 thumbnails; lazy-load the rest on scroll
✅ Done when: PDF with 5+ pages shows thumbnails in sidebar strip

? ### Day 7 — Single-page viewer
**Phase 1 | Week 2**
- Add `render_page(path: String, page_index: usize, dpi: f32) -> Result<String, String>` command:
  - Render at specified DPI (default 144 for retina, 72 for standard)
  - Write to temp PNG file, return path
- Frontend: `PageViewer` component with:
  - Full-size page render displayed as `<img>`
  - Zoom controls: fit-to-width, fit-to-page, 50%, 75%, 100%, 150%, 200%
  - Previous/next page buttons; page number input (jump to page)
- Debounce re-render on zoom change (wait 300ms after last change)
✅ Done when: can navigate and zoom a multi-page PDF

? ### Day 8 — PDF error handling + edge cases
**Phase 1 | Week 2**
- Handle password-protected PDFs: catch PDFium error, return `Err("PDF is encrypted — password required")`
- Handle corrupted/truncated PDFs: catch parse errors, return descriptive error
- Handle zero-page PDFs: guard against empty page tree
- Handle PDFs with non-standard Info dictionary (missing fields default to `""`)
- Frontend: show inline error banner (not alert()) when `open_pdf` fails; don't crash the view
- Test with: a password-protected PDF, a truncated PDF, a zero-byte file renamed to `.pdf`
✅ Done when: all three error cases show a useful error message in the UI

? ### Day 9 — Recent files + cleanup
**Phase 1 | Week 2**
- Limit `pdf_jobs` to 20 most recent (delete oldest on insert when count > 20)
- Add "Remove from history" button on each recent file row
- Add "Open Again" button that re-runs `open_pdf` with stored path
- Handle case where stored file path no longer exists (show "File not found" state)
- Sort recent files by `opened_at` descending
✅ Done when: recent files list is usable and self-managing

? ### Day 10 — Phase 1.1 cleanup + PR
**Phase 1 | Week 2**
- Run `cargo check` and `npx tsc --noEmit` — fix all warnings and errors
- Test with 5 real PDFs: simple, multi-page, encrypted, large (100+ pages), small (1 page, no metadata)
- Commit all PDF ingestion layer work
- Push branch, create PR: "Add PDF viewer and job history (Phase 1.1)"
- Merge to main
✅ Done when: clean green PR merged; PDF Tools section shows thumbnails and metadata for any PDF

---

? ### Day 11 — Font data structures
**Phase 1 | Week 3**
- Add to `models.rs`:
  ```rust
  pub struct FontFinding {
      pub font_name: String,
      pub font_type: String,       // "Type1", "TrueType", "CIDFont", "OpenType"
      pub is_embedded: bool,
      pub is_subsetted: bool,
      pub pages: Vec<usize>,       // 1-indexed page numbers where this font appears
      pub severity: String,        // "error" | "warning" | "ok"
      pub message: String,
  }
  ```
- Write `pdf::fonts::collect_fonts(doc: &lopdf::Document) -> Vec<FontFinding>` stub
✅ Done when: struct compiles and stub returns empty vec

? ### Day 12 — Font enumeration via PDFium
**Phase 1 | Week 3**
- Implement font walking: for each page, get `PdfPageFonts` via pdfium-render
- For each font on the page: `font.is_embedded()`, `font.family_name()`, `font.font_type()`
- Detect subset prefix: font name matches regex `^[A-Z]{6}\+` → subsetted
- Collect per-font: deduplicate by name, accumulate page list
✅ Done when: test PDF with known fonts shows correct embedded status

? ### Day 13 — lopdf fallback for font descriptor
**Phase 1 | Week 3**
- Some PDFs expose fonts via the raw dictionary that PDFium doesn't surface
- Write lopdf fallback: walk `Page.Resources.Font` dictionary, check `FontDescriptor` for `FontFile`, `FontFile2`, or `FontFile3` keys
- Cross-reference with PDFium results: if lopdf finds a font PDFium missed, add it
- Handle Type0 (composite) fonts: check their `DescendantFonts` array entry
✅ Done when: font list is complete including CIDFonts used in CJK documents

? ### Day 14 — `check_fonts` command + severity assignment
**Phase 1 | Week 3**
- Write `commands::check_fonts(path: String) -> Result<Vec<FontFinding>, String>`
- Severity rules:
  - Not embedded → `"error"` + message "Font '{name}' is not embedded. The receiving printer may substitute a different font."
  - Embedded but not subsetted → `"warning"` + message "Font '{name}' is fully embedded (not subsetted). Consider subsetting to reduce file size."
  - Embedded and subsetted → `"ok"`
- Register command in `lib.rs`
✅ Done when: `check_fonts` returns correct severity for each font in test PDFs

? ### Day 15 — FontCheck UI component
**Phase 1 | Week 3**
- Create `src/components/preflight/FontCheck.tsx`:
  - Summary row: N fonts found, N errors, N warnings
  - Table: font name | type | embedded | subsetted | pages | status badge
  - Filter tabs: All / Issues Only
  - Status badge colors: red (error), yellow (warning), green (ok)
- Wire into PDFView as a collapsible check section
- Add `FontCheck.css`
✅ Done when: running font check on a PDF with mixed embedded/unembedded fonts shows correct table

? ### Day 16 — PageBox data structures + lopdf extraction
**Phase 1 | Week 4**
- Add to `models.rs`:
  ```rust
  pub struct PageBox { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
  pub struct PageBoxFinding {
      pub page: usize,
      pub media_box: Option<PageBox>,
      pub trim_box: Option<PageBox>,
      pub bleed_box: Option<PageBox>,
      pub art_box: Option<PageBox>,
      pub crop_box: Option<PageBox>,
      pub issues: Vec<String>,
      pub severity: String,
  }
  ```
- Write lopdf box extraction: read page dictionary, handle inherited values (walk up parent nodes)
- Convert from PDF points to mm: `points × 0.3528`
✅ Done when: extracting boxes from a PDF returns correct mm values matching Acrobat's display

? ### Day 17 — Page box validation logic
**Phase 1 | Week 4**
- Validate each page:
  - MediaBox missing → error
  - TrimBox missing → warning (required for PDF/X)
  - BleedBox present but smaller than TrimBox on any side → error
  - BleedBox absent when TrimBox present → info (not an error unless PDF/X)
  - TrimBox larger than MediaBox → error
  - Mixed page sizes in document → warning (flag inconsistent pages)
- Bundle standard bleed sizes: 3mm (ISO), 0.125" (US), configurable
✅ Done when: validation correctly flags a known-bad PDF provided as test case

? ### Day 18 — `check_page_boxes` command
**Phase 1 | Week 4**
- Write `commands::check_page_boxes(path: String) -> Result<Vec<PageBoxFinding>, String>`
- Aggregate document-level summary: all pages pass / N pages have issues
- Register in `lib.rs`
✅ Done when: command returns per-page findings with correct severity

? ### Day 19 — PageBoxCheck UI component
**Phase 1 | Week 4**
- Create `src/components/preflight/PageBoxCheck.tsx`:
  - Per-page rows: page number, boxes present (checkmarks), any issues
  - Click page row to expand: show all box dimensions in mm
  - Visual diagram: nested rectangles showing MediaBox > BleedBox > TrimBox relationship (SVG)
  - Summary: "All pages have correct bleed" or "3 pages missing TrimBox"
✅ Done when: component renders box diagram and per-page issue list

? ### Day 20 — Page box edge cases + PR
**Phase 1 | Week 4**
- Handle: boxes defined via indirect object reference (lopdf `Document::dereference`)
- Handle: page tree nodes with Resources inherited across page subtrees
- Handle: rotated pages (swap width/height when Rotate = 90 or 270)
- Run tests, cargo check, tsc
- PR: "Add font embedding and page box preflight checks"
✅ Done when: clean PR merged

---

? ### Day 21 — Graphics state machine skeleton
**Phase 1 | Week 5**
- Create `src-tauri/src/pdf/content_stream.rs`
- Define `GraphicsState` struct:
  ```rust
  struct GraphicsState {
      ctm: [f64; 6],        // current transformation matrix [a b c d e f]
      stack: Vec<[f64; 6]>, // q/Q stack
  }
  impl GraphicsState {
      fn identity() -> Self { ... }
      fn push(&mut self) { self.stack.push(self.ctm) }
      fn pop(&mut self) { if let Some(m) = self.stack.pop() { self.ctm = m } }
      fn apply_cm(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) { ... }
  }
  ```
- Implement matrix multiplication for `apply_cm`
✅ Done when: CTM correctly multiplies and stacks/unstacks in unit tests

? ### Day 22 — Content stream byte tokenizer
**Phase 1 | Week 5**
- Write `tokenize(bytes: &[u8]) -> Vec<Token>` in `content_stream.rs`
- `Token` enum: `Number(f64)`, `Name(String)`, `StringLit(Vec<u8>)`, `Operator(String)`, `ArrayStart`, `ArrayEnd`, `DictStart`, `DictEnd`
- Handle: comments (`% ...` to end of line), hex strings (`<...>`), literal strings (`(...)`), names (`/Name`)
- Handle: operator tokenization — letters only, no numbers, e.g., `cm`, `Do`, `Tf`, `q`, `Q`, `rg`, `k`
✅ Done when: tokenizer correctly parses a sample content stream extracted from a real PDF

? ### Day 23 — CTM tracking through content stream
**Phase 1 | Week 5**
- Write `execute_stream(tokens: &[Token], state: &mut GraphicsState)`
- Handle operators that affect CTM:
  - `q` → push state
  - `Q` → pop state
  - `cm a b c d e f` → apply_cm with 6 preceding numbers
- Collect operand stack: numbers and names preceding each operator
✅ Done when: running `execute_stream` on a content stream with nested `q`/`Q`/`cm` produces correct CTM values

? ### Day 24 — `Do` operator image tracking
**Phase 1 | Week 5**
- On encountering `Do /XObjectName`:
  - Record: XObject name, current CTM at time of invocation
  - Look up in page Resources/XObject dictionary
  - If Subtype = Image: retrieve `Width` and `Height` keys
  - Calculate rendered width/height from CTM: `rendered_w = sqrt(a² + c²) × (Width_pts / 72)`
- Store: `ImageUsage { xobject_name, pixel_width, pixel_height, ctm, effective_dpi }`
✅ Done when: test PDF with a known image shows correct pixel dimensions and rendered size

? ### Day 25 — DPI calculation + `ImageResolutionFinding`
**Phase 1 | Week 5**
- Add to `models.rs`:
  ```rust
  pub struct ImageResolutionFinding {
      pub page: usize,
      pub xobject_name: String,
      pub pixel_width: u32,
      pub pixel_height: u32,
      pub rendered_width_mm: f64,
      pub rendered_height_mm: f64,
      pub effective_dpi: f64,
      pub color_space: String,
      pub severity: String,     // "error" | "warning" | "ok"
      pub message: String,
  }
  ```
- DPI calculation: `effective_dpi = (pixel_width / (rendered_width_pts / 72.0))`
- Severity: below 150 DPI → error; 150–299 → warning; 300+ → ok (all configurable)
✅ Done when: struct compiles and DPI is correctly calculated for a test image

? ### Day 26 — `check_image_resolution` command
**Phase 1 | Week 6**
- Write `commands::check_image_resolution(path: String, min_dpi: f64) -> Result<Vec<ImageResolutionFinding>, String>`
  - Iterate pages, run content stream parser on each
  - Collect all `Do` operator image usages with CTM
  - Look up XObject in lopdf document, extract Width/Height/ColorSpace
  - Apply DPI check
- Register in `lib.rs`
✅ Done when: returns correct findings for a PDF with known-low-DPI images

### Day 27 — ImageResolutionCheck UI component
**Phase 1 | Week 6**
- Create `src/components/preflight/ImageResolutionCheck.tsx`:
  - Configurable min DPI slider (common values: 72, 150, 300, 600)
  - Summary: N images checked, N below threshold
  - Table: page | image ID | pixel dimensions | rendered size mm | DPI | color space | status
  - Sort by DPI ascending (worst first)
✅ Done when: UI correctly shows DPI table and re-runs check when threshold slider changes

### Day 28 — Form XObjects and inline images
**Phase 1 | Week 6**
- Form XObjects: when `Do /Name` and XObject Subtype = Form (not Image), recurse into its content stream
  - Apply the Form's own Matrix key to the current CTM before recursing
  - Track nested recursion depth, bail at depth > 10 (malformed PDF protection)
- Inline images (`BI ... EI`): parse inline image dict to get Width/Height/ColorSpace, use current CTM
✅ Done when: PDF using Form XObjects to wrap images still reports correct DPI

### Day 29 — Images used multiple times
**Phase 1 | Week 6**
- Same XObject can appear via multiple `Do` calls at different positions/sizes
- Report each usage independently (same image at 72 DPI in one place and 300 DPI in another → two rows)
- Deduplicate identical usages at same CTM (PDF generators sometimes emit duplicate `Do` operators)
✅ Done when: multi-use image in test PDF shows each distinct usage as separate row

### Day 30 — DPI performance + PR
**Phase 1 | Week 6**
- Performance test: run `check_image_resolution` on a 100-page PDF with 200+ images
- If > 5 seconds: parallelize page processing using `rayon` crate (`par_iter` over pages)
- Add `rayon = "1"` to Cargo.toml if needed
- Run cargo check + tsc
- PR: "Add image DPI resolution check (Phase 1.3)"
✅ Done when: 100-page PDF checks in < 3 seconds; PR merged

---

### Day 31 — Bleed validation logic
**Phase 1 | Week 7**
- Write `pdf::bleed::check_bleed(doc: &lopdf::Document, min_bleed_mm: f64) -> Vec<BleedFinding>`
- Add `BleedFinding` to models.rs: page, has_bleed_box, bleed_top_mm, bleed_right_mm, bleed_bottom_mm, bleed_left_mm, min_required_mm, severity
- Measure bleed on each of 4 sides: `(BleedBox.coord - TrimBox.coord) × 0.3528`
- Flag each side independently: "Right bleed is 2.1mm — minimum is 3mm"
✅ Done when: correctly identifies a PDF with bleed on 3 sides but not 4th

### Day 32 — `add_bleed` fixup command
**Phase 1 | Week 7**
- Write `commands::add_bleed(path: String, amount_mm: f64, output_path: String) -> Result<(), String>`:
  - Load with lopdf
  - For each page: expand BleedBox by `amount_mm` on all 4 sides (if no BleedBox, derive from TrimBox)
  - Expand MediaBox to contain new BleedBox if needed
  - Write modified PDF to `output_path` (never overwrite input)
- Add to `lib.rs`
✅ Done when: applying add_bleed to a file opens correctly in Acrobat with correct box sizes

### Day 33 — BleedCheck UI component
**Phase 1 | Week 7**
- Create `src/components/preflight/BleedCheck.tsx`:
  - Per-page table: page | bleed top | right | bottom | left | status
  - Side-specific highlighting: show which sides are insufficient
  - SVG diagram: page box visualization with bleed margin shown in red/green
  - "Add Bleed" fixup panel: amount input (mm), output filename, run button
✅ Done when: bleed check and fixup work end-to-end from the UI

### Day 34 — `check_bleed` command + wire to UI
**Phase 1 | Week 7**
- Write `commands::check_bleed(path: String, min_bleed_mm: f64) -> Result<Vec<BleedFinding>, String>`
- Register in `lib.rs`
- Wire BleedCheck component: calls check_bleed on load, shows add_bleed button if any failures
✅ Done when: full round-trip works (check → fixup → re-check shows pass)

### Day 35 — Bleed edge cases + PR
**Phase 1 | Week 7**
- Handle PDFs with ArtBox instead of TrimBox (treat ArtBox as trim reference)
- Handle PDFs where page rotation affects which is "top" vs. "right" bleed
- Test: no bleed at all, correct bleed, partial bleed, bleed box smaller than trim (corrupted)
- PR: "Add bleed detection and add-bleed fixup (Phase 1.4)"
✅ Done when: PR merged

---

### Day 36 — OutputIntent extraction
**Phase 1 | Week 8**
- Write `pdf::metadata::get_output_intents(doc: &lopdf::Document) -> Vec<OutputIntent>`
- `OutputIntent` struct: s_key (GTS_PDFX / GTS_PDFA), output_condition, output_condition_id, registry_name, has_embedded_icc, icc_num_channels
- Walk document catalog → `OutputIntents` array → each dictionary entry
- Check `DestOutputProfile` stream presence
✅ Done when: correctly parses OutputIntents from a PDF/X-1a file

### Day 37 — PDF/X metadata checks
**Phase 1 | Week 8**
- Write `pdf::pdfx::check_metadata(doc: &lopdf::Document) -> Vec<PdfXFinding>`:
  - Check `GTS_PDFXVersion` key in Info dictionary
  - Parse and validate version string
  - Check `Trapped` key in Info dictionary (must be `/True`, `/False`, or `/Unknown`)
  - Check PDF version from header (X-1a requires ≥ 1.3, X-4 requires ≥ 1.6)
✅ Done when: metadata check correctly identifies missing or malformed keys

### Day 38 — Security and forbidden content checks
**Phase 1 | Week 8**
- Check `Encrypt` key in trailer dictionary → error if present
- Scan all page annotations: flag Sound, Movie, Screen, Widget (unless non-printing)
- Check for JavaScript: `Names` catalog entry → `JavaScript` name tree; `AA` (additional actions) on pages
- Check for OPI: scan XObject dictionaries for `/OPI` key
✅ Done when: a PDF with JavaScript returns an error for that specific check

### Day 39 — PDF/X-1a compliance assembler
**Phase 1 | Week 8**
- Write `commands::check_pdfx(path: String, profile: String) -> Result<Vec<PdfXFinding>, String>`
- For "x1a": run all of:
  - `check_fonts` (all embedded)
  - `check_page_boxes` (TrimBox or ArtBox on all pages)
  - `check_output_intent` (OutputIntents array with GTS_PDFX)
  - `check_metadata` (GTS_PDFXVersion, Trapped)
  - `check_security` (no encryption, no JS, no multimedia)
  - Color check stub: add finding "⚠ Color space validation runs in Phase 2"
- Return combined, deduplicated finding list
✅ Done when: checking a known PDF/X-1a file returns mostly-pass with color stub warning

### Day 40 — PDF/X-3 and X-4 profiles
**Phase 1 | Week 8**
- X-3: same as X-1a but mark color check as "ICC-managed RGB allowed" (still stub)
- X-4: same as X-3 but remove transparency check (live transparency allowed)
- Write `profile: "x1a" | "x3" | "x4"` parameter dispatching
✅ Done when: three profiles produce different findings on the same transparency-containing file

### Day 41 — Deep PDF inspector UI (#29)
**Phase 1 | Week 9**
- Create `src/components/preflight/PdfInspector.tsx`:
  - Object browser: show document catalog as expandable tree
  - Each object reference clickable → shows raw dictionary or stream info
  - Tab: "Document Info" (Info dict fields), "Page Tree" (page count, page size summary), "Resources" (fonts, XObjects, colorspaces found in document)
- New command: `get_pdf_catalog(path: String) -> Result<serde_json::Value, String>` returning catalog dict as JSON
✅ Done when: inspector shows the document catalog keys for any PDF

### Day 42 — Inspector page-level detail
**Phase 1 | Week 9**
- Add page selector to inspector: click page number → show that page's dictionary
- Show per-page: Resources/Font, Resources/XObject, Resources/ColorSpace, MediaBox, TrimBox
- Show page stream length, compression filter
- "View Raw Stream" button for any XObject or content stream (shows decoded bytes as text)
✅ Done when: inspector shows per-page font and resource details

### Day 43 — `preflight_findings` DB table (#30)
**Phase 1 | Week 9**
- Add to `db.rs`:
  ```sql
  CREATE TABLE IF NOT EXISTS preflight_findings (
    id INTEGER PRIMARY KEY,
    job_id INTEGER NOT NULL REFERENCES pdf_jobs(id) ON DELETE CASCADE,
    check_name TEXT NOT NULL,
    severity TEXT NOT NULL,
    page_num INTEGER,
    object_ref TEXT,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL
  )
  ```
- Add `preflight_run_summary` table: job_id, profile, total_errors, total_warnings, total_ok, ran_at
- Write `db::save_findings(job_id, findings: Vec<&dyn AsFindings>)`
✅ Done when: running a preflight check persists findings to DB

### Day 44 — `list_findings_for_job` command + history
**Phase 1 | Week 9**
- Write `commands::list_findings_for_job(job_id: i64) -> Result<Vec<PreflightFinding>, String>`
- Write `commands::list_preflight_runs(job_id: i64) -> Result<Vec<PreflightRunSummary>, String>`
- Frontend: `PreflightHistory` component showing past runs with pass/fail badge and timestamp
- Click run → reload stored findings (don't re-run the check)
✅ Done when: running preflight twice on the same file shows two entries in history

### Day 45 — Combined preflight report UI
**Phase 1 | Week 9**
- Create `src/components/preflight/PreflightReport.tsx`:
  - Top summary banner: PASS (all green) or FAIL (N errors, N warnings) with color
  - Sections: Font Checks | Page Boxes | Bleed | PDF/X Metadata | Security
  - Each section collapsible with section-level pass/fail badge
  - Each finding row: severity icon, check name, page (if applicable), message
  - "Run Full Check" button with profile selector (X-1a / X-3 / X-4 / Custom)
✅ Done when: combined report renders all check categories in one scrollable view

### Day 46 — Report auto-save on run
**Phase 1 | Week 10**
- When "Run Full Check" completes: auto-save summary + all findings to DB under this `pdf_job` record
- Show "Saved at [timestamp]" confirmation
- Allow running again: creates a new `preflight_run_summary` row, preserves history
✅ Done when: report persists and reloads correctly after app restart

### Day 47 — Error count badges in sidebar
**Phase 1 | Week 10**
- Show error/warning count badge next to "PDF Tools" in ManagementView sidebar
- Pull from most recent `preflight_run_summary` for the currently open file
- Badge: red circle with number for errors, yellow for warnings (like macOS App Store badge)
✅ Done when: opening a bad PDF and running preflight shows red badge in nav

### Day 48 — PDF/X finding messages polish
**Phase 1 | Week 10**
- Audit all finding messages for clarity and actionability:
  - Each error includes: what it means, what will happen at print, how to fix it in InDesign/Illustrator
  - Example: "Font 'Helvetica' is not embedded. If the printer doesn't have this font, text will reflow or substitute. To fix: in InDesign, go to File → Export → PDF → Advanced and check 'Embed Fonts'. In Illustrator, use Type → Create Outlines before exporting."
- Add `fix_hint` field to all Finding structs
✅ Done when: every error in the report has a practical fix hint

### Day 49 — Keyboard shortcuts for PDF view
**Phase 1 | Week 10**
- Arrow left/right: previous/next page
- `+` / `-`: zoom in/out
- `Cmd/Ctrl+O`: open file picker
- `Cmd/Ctrl+R`: run preflight
- `Escape`: close expanded finding
- Add keyboard shortcut hints to button tooltips
✅ Done when: navigating a PDF entirely by keyboard works smoothly

### Day 50 — Phase 1 full integration test + PR
**Phase 1 | Week 10**
- Gather 10 real-world PDFs (mix of good, bad, PDF/X-1a, RGB, missing fonts, no bleed)
- Run full preflight on each; verify findings match what Acrobat Pro reports
- Fix any discrepancies found
- `cargo check`, `npx tsc --noEmit`, no errors
- Final PR: "Phase 1 complete — PDF preflight foundation"
- Merge to main
✅ Done when: Phase 1 PR merged; app can open, inspect, and preflight any PDF

---

## Phase 2 — Color Space Detection
*Issues: #23, #25, #26, #34 | 40 days*

---

### Day 51 — Content stream operator taxonomy
**Phase 2 | Week 11**
- Extend `content_stream.rs` with full operator set for color:
  - Stroke color: `CS` (set color space), `SC`/`SCN` (set color value), `G` (gray), `RG` (RGB), `K` (CMYK)
  - Fill color: `cs`, `sc`/`scn`, `g`, `rg`, `k`
  - Named color spaces: resolved from `Resources/ColorSpace` dictionary
  - Extended graphics state: `gs /Name` → look up in `Resources/ExtGState`
- Write unit tests for each operator type
✅ Done when: all color operators are tokenized correctly from a sample stream

### Day 52 — Color space resolver
**Phase 2 | Week 11**
- Write `pdf::color::resolve_color_space(name: &str, resources: &lopdf::Dictionary) -> ColorSpaceKind`
- `ColorSpaceKind` enum: `DeviceGray`, `DeviceRGB`, `DeviceCMYK`, `CalGray`, `CalRGB`, `Lab`, `ICCBased(icc_channels)`, `Separation(alt_space)`, `DeviceN(alt_space)`, `Indexed(base)`, `Pattern`, `Unknown`
- Handle: array form `[/ICCBased stream_ref]`, name form `/DeviceRGB`, indirect references
- Recursively resolve base spaces for Indexed and Separation
✅ Done when: resolver correctly identifies ICC-based RGB vs. DeviceCMYK from Resources dict

### Day 53 — Color usage accumulator
**Phase 2 | Week 11**
- Write `ColorUsage` struct: color_space_kind, is_stroke, is_fill, page, object_context
- Extend `execute_stream` to populate a `Vec<ColorUsage>` as it processes operators
- Track: current stroke color space, current fill color space, update on each color operator
- Also handle: form XObjects (recurse, passing their own Resources dict)
✅ Done when: processing a mixed RGB+CMYK PDF returns usages of both color spaces

### Day 54 — ColorSpaceFinding struct + classification
**Phase 2 | Week 12**
- Add `ColorSpaceFinding` to models.rs: color_space, kind (stroke/fill), pages, is_pdf_x_violation, severity, message
- PDF/X-1a violations: any DeviceRGB, CalRGB, Lab usage → error
- PDF/X-3 violations: any RGB/Lab without embedded ICC profile → error
- Non-compliant but not PDF/X: RGB in a job → warning ("Job appears to be CMYK-intended but contains RGB objects")
✅ Done when: finding classification correctly distinguishes X-1a vs. X-3 violations

### Day 55 — `check_color_spaces` command
**Phase 2 | Week 12**
- Write `commands::check_color_spaces(path: String, target_profile: String) -> Result<Vec<ColorSpaceFinding>, String>`
- `target_profile`: "pdfx_1a", "pdfx_3", "pdfx_4", "cmyk_only", "any"
- Aggregate: list all color spaces found, highlight violations per profile
- Register in `lib.rs`
✅ Done when: correctly identifies an RGB image in a file exported with "CMYK only" target

### Day 56 — Integrate color check into PDF/X compliance
**Phase 2 | Week 12**
- Remove color check stub from `check_pdfx` (added Day 39)
- Wire real `check_color_spaces` result into the assembled PDF/X report
- PDF/X-1a: any DeviceRGB → error; any ICCBased color → error
- PDF/X-3: ICCBased color allowed; DeviceRGB without ICC → error
- PDF/X-4: same as X-3
✅ Done when: a known-bad RGB-containing PDF/X export shows color errors in the report

### Day 57 — ColorSpaceCheck UI component
**Phase 2 | Week 12**
- Create `src/components/preflight/ColorSpaceCheck.tsx`:
  - Summary: color spaces found (list of chips: DeviceCMYK ✓, DeviceRGB ✗, ICCBased ✓)
  - Table: color space | type | stroke/fill | pages | compliant with [profile]
  - Highlight violations in red
  - Show ICC profile info for ICCBased spaces: profile name, channels, colorspace type
✅ Done when: component correctly shows mixed RGB+CMYK usage breakdown

### Day 58 — ICC profile info extraction
**Phase 2 | Week 12**
- For `ICCBased` color spaces: read the ICC profile stream bytes
- Parse ICC profile header (first 128 bytes): color space signature (4 bytes at offset 16), profile class
- Extract: number of channels (N key in PDF dict), color space type (from profile header bytes)
- ICC profile color space signatures: `RGB ` = 0x52474220, `CMYK` = 0x434D594B, `GRAY` = 0x47524159
- Show profile description if present (tag `desc`)
✅ Done when: correctly identifies an sRGB vs FOGRA39 ICC profile from their binary headers

### Day 59 — Overprint detection (#25)
**Phase 2 | Week 13**
- Extend `execute_stream`: track `ExtGState` lookups from `gs` operator
- For each `ExtGState` referenced:
  - Check `OP` key (overprint stroke) and `op` key (overprint fill)
  - Check `OPM` (overprint mode: 0 = knockout, 1 = non-zero rule)
- Create `OverprintFinding`: page, object_context, overprint_stroke, overprint_fill, mode, severity, message
- Severity rules:
  - 0% CMYK with overprint enabled → error ("White knockout missing — 0% ink with overprint will show through to underlying ink")
  - Overprint enabled on RGB color space → error (meaningless, will be ignored by RIP)
  - Overprint enabled on CMYK (expected K-only black) → info
✅ Done when: finds known overprint issue in test PDF

### Day 60 — `check_overprint` command + UI
**Phase 2 | Week 13**
- Write `commands::check_overprint(path: String) -> Result<Vec<OverprintFinding>, String>`
- Register in `lib.rs`
- Add OverprintCheck section to PreflightReport (collapsible)
- Show "Overprint Preview" toggle in page viewer (future: actual simulation in Phase 3)
✅ Done when: overprint findings appear in the combined preflight report

### Day 61 — Transparency detection (#25)
**Phase 2 | Week 13**
- Check for transparency groups: presence of `Group` dictionary in page dictionary
- Scan `ExtGState` entries across all pages for: `ca` (fill opacity), `CA` (stroke opacity), `BM` (blend mode)
- Any value of `ca` or `CA` < 1.0 → live transparency present
- Any `BM` value other than `/Normal` and `/Compatible` → live transparency present
- `TransparencyFinding`: page, type (opacity/blend_mode), value, is_pdfx1a_violation, severity
✅ Done when: InDesign drop-shadow PDF correctly flags live transparency

### Day 62 — `check_transparency` command + UI
**Phase 2 | Week 13**
- Write `commands::check_transparency(path: String) -> Result<Vec<TransparencyFinding>, String>`
- Register in `lib.rs`
- Add to PreflightReport transparency section
- PDF/X-1a: any transparency → error; PDF/X-4: info only
✅ Done when: transparency check integrates into PDF/X-1a compliance failure correctly

### Day 63 — Hidden content detection (#26)
**Phase 2 | Week 14**
- Detect content outside the visible area (MediaBox clipping):
  - Find text/path objects placed outside the MediaBox
  - This requires tracking path coordinates from `m`/`l`/`c` operators and text positions from `Td`/`TD`/`Tm`
- Detect off-page objects: `ObjectFinding` with approximate location
- Check Optional Content Groups (layers): find layers that are default-off
- White objects on white background (requires color + graphics state tracking)
✅ Done when: a PDF with off-page crop marks correctly reports hidden content

### Day 64 — `check_hidden_content` command + UI
**Phase 2 | Week 14**
- Write `commands::check_hidden_content(path: String) -> Result<Vec<HiddenContentFinding>, String>`
- Register in `lib.rs`
- Hidden content findings in report: page, type (off-page / default-off-layer / white-on-white), description
✅ Done when: hidden content section appears in report

### Day 65 — lcms2 FFI setup
**Phase 2 | Week 14**
- Add `lcms2-sys = "0.7"` to Cargo.toml
- Verify lcms2 compiles on Windows (may require `vcpkg` or bundled C source)
- Write `pdf::color::transforms::LcmsEngine` wrapper:
  - `open_profile_from_bytes(bytes: &[u8]) -> Profile`
  - `create_transform(src: Profile, dst: Profile, intent: RenderingIntent) -> Transform`
  - `transform_pixels(transform: &Transform, src: &[u8], channels_in: u8) -> Vec<u8>`
- Write smoke test: transform a known RGB triple to CMYK using sRGB → FOGRA39
✅ Done when: `cargo check` passes with lcms2; smoke test produces expected CMYK values

### Day 66 — ICC profile bundling
**Phase 2 | Week 14**
- Bundle standard ICC profiles as Tauri resources:
  - `sRGB_v4_ICC_preference.icc` (sRGB IEC 61966-2-1)
  - `ISOcoated_v2_eci.icc` (ISO Coated v2 / FOGRA39 — most common offset printing)
  - `USWebCoatedSWOP.icc` (US Web Coated SWOP — US market)
  - `GRACoL2006_Coated1v2.icc` (GRACoL — US sheet-fed)
- Write `commands::list_icc_profiles() -> Result<Vec<IccProfileInfo>, String>`
- `IccProfileInfo`: name, description, color_space, channels, file_name
✅ Done when: bundled profiles load via lcms2 without error

### Day 67 — `convert_rgb_to_cmyk` fixup command
**Phase 2 | Week 15**
- Write `commands::convert_rgb_to_cmyk(path: String, src_profile: String, dst_profile: String, rendering_intent: String, output_path: String) -> Result<(), String>`:
  - Load PDF with lopdf
  - Find all image XObjects with DeviceRGB or ICCBased-RGB color space
  - For each image: decode stream bytes, apply lcms2 transform, re-encode as JPEG (quality 90)
  - Update image XObject: change ColorSpace to `/DeviceCMYK`, update stream bytes
  - Update image stream Length key
  - Write output file
- Register in `lib.rs`
✅ Done when: a simple RGB-only PDF converts to CMYK and opens correctly in Acrobat

### Day 68 — Vector color conversion
**Phase 2 | Week 15**
- For non-image (vector) objects with DeviceRGB color: convert color values inline in content stream
  - Find `rg R G B` operators → replace with `k C M Y K` operators using lcms2 transform
  - Convert each sampled RGB triple to CMYK via the selected transform
- This requires content stream round-trip: tokenize → modify → re-encode bytes
- Re-encode: serialize tokens back to byte stream, update stream dict (remove filter or re-apply)
✅ Done when: a PDF with RGB text and fills converts vector colors to CMYK

### Day 69 — Color conversion UI + profile selector
**Phase 2 | Week 15**
- Create `src/components/preflight/ColorConversionPanel.tsx`:
  - Source profile dropdown (auto-detected from file, or select override)
  - Destination profile dropdown (FOGRA39, SWOP, GRACoL, or custom — browse for ICC file)
  - Rendering intent: Perceptual (default) | Relative Colorimetric | Absolute Colorimetric | Saturation
  - Scope: Images only / Vector only / Both
  - Output file path (defaults to `[original_name]_CMYK.pdf`)
  - "Convert Colors" button with progress indicator
✅ Done when: end-to-end RGB→CMYK conversion runs from the UI

### Day 70 — Color conversion edge cases
**Phase 2 | Week 15**
- Handle: images with indexed color spaces (resolve base space first, then convert)
- Handle: images with Lab color (Lab → CMYK via lcms2)
- Handle: spot colors (Separation/DeviceN) — leave unchanged, flag as "Spot color: [name] — not converted"
- Handle: content streams using named resources from global Resources dict vs. page-level
✅ Done when: mixed spot + RGB PDF converts RGB/Lab components, leaves spots intact

### Day 71 — ICC profile embedding into OutputIntent
**Phase 2 | Week 16**
- Write `commands::add_output_intent(path: String, icc_profile: String, condition_id: String, output_path: String) -> Result<(), String>`
- Create OutputIntents array in document catalog (if absent)
- Embed ICC profile as stream object
- Set required keys: `/S /GTS_PDFX`, `/OutputConditionIdentifier`, `/DestOutputProfile`
- Set `/GTS_PDFXVersion` in Info dictionary if not already present
✅ Done when: adding OutputIntent to a file makes it pass the OutputIntent check

### Day 72 — "Make PDF/X-1a" wizard
**Phase 2 | Week 16**
- Multi-step UI wizard:
  1. Run full preflight → show issues
  2. For each auto-fixable issue: toggle on/off (font subsetting — skip; add bleed — toggle; convert colors — toggle; add OutputIntent — toggle)
  3. Review: show summary of what will be changed
  4. Apply all selected fixups in sequence → write output file
  5. Auto-run preflight on output file → show new results
- This is the flagship fixup workflow
✅ Done when: a typical InDesign-exported RGB file goes through wizard and comes out as valid PDF/X-1a (minus font subsetting, which is Phase 5)

### Day 73 — Phase 2 color check integration
**Phase 2 | Week 16**
- Wire all color checks into combined PreflightReport:
  - Color Spaces (with ICC info)
  - Overprint
  - Transparency
  - Hidden Content
- Each section expandable, each finding with fix hint
- Total error/warning count updates dynamically
✅ Done when: full preflight report runs all Phase 1 + Phase 2 checks in one click

### Day 74 — Rendering intent explanation + help text
**Phase 2 | Week 16**
- Add tooltips/help text to color conversion UI:
  - Perceptual: "Best for photos — compresses gamut to fit destination, maintains visual relationships"
  - Relative Colorimetric: "Best for logos and spot color approximations — shifts white point, clips out-of-gamut colors"
  - Absolute Colorimetric: "Preserves absolute color values — only use when simulating one press on another"
  - Saturation: "Best for business graphics — maximizes saturation, not accurate for critical color"
✅ Done when: all tooltips are present and accurate

### Day 75 — Color conversion performance
**Phase 2 | Week 17**
- Profile: convert a 50-page PDF with 30 high-res images
- If > 30 seconds: parallelize image conversion using `rayon`
- Ensure lopdf write is single-threaded (not thread-safe); collect all converted streams then write
- Add progress callback via Tauri event: `emit("conversion_progress", { current: n, total: N })`
✅ Done when: progress bar shows during conversion; large file converts in reasonable time

### Day 76 — `check_ink_coverage` stub
**Phase 2 | Week 17**
- Add `InkCoverageFinding` to models.rs: page, max_tac (total area coverage), average_tac, exceeds_threshold
- Write stub command `check_ink_coverage` that returns `Err("Ink coverage requires rendering — available in Phase 5")`
- Add placeholder section in PreflightReport with "Coming in Phase 5" banner
- This reserves the API shape so the UI doesn't need to change later
✅ Done when: stub compiles and shows placeholder in report

### Day 77 — Color check for non-PDF/X jobs
**Phase 2 | Week 17**
- Not all print jobs need PDF/X. Add a "General Print" profile:
  - Flag RGB images as warnings (not errors)
  - Flag total ink coverage > 300% as warning
  - Flag spot colors without defined alternates as warning
  - No requirements on OutputIntent, Trapped key, etc.
- This is the right default profile for a small print shop
✅ Done when: "General Print" profile produces sensible warnings for a typical RGB flyer file

### Day 78 — Spot color inventory
**Phase 2 | Week 17**
- Walk all Separation and DeviceN color spaces in the document
- Extract spot color names (e.g., "PANTONE 485 C", "Die Cut", "Varnish")
- `SpotColorFinding`: name, pages it appears on, has_alternate_colorspace, alternate_colorspace_type
- Flag: spot colors named "Cut" / "Die" / "Crease" / "Varnish" are likely special process colors — flag for review
✅ Done when: PDF with PANTONE spot colors correctly lists them all by name

### Day 79 — Spot color UI + PR
**Phase 2 | Week 18**
- Add SpotColorInventory section to report: table of spot colors with page usage and alternate space type
- Highlight special-purpose spot colors (die/varnish/cut) in a distinct color
- PR: "Phase 2 complete — color space detection and conversion"
✅ Done when: PR merged

### Day 80 — Phase 2 regression test
**Phase 2 | Week 18**
- Run Phase 1 + Phase 2 preflight on the 10 test PDFs from Day 50
- Compare results with Acrobat Pro / pdfToolbox findings for same files
- Document any discrepancies
- Fix the top 3 discrepancies found
✅ Done when: results match reference tool on 8/10 files; known discrepancies documented

---

## Phase 3 — PDF Viewing and Editing Foundations
*Issues: #31, #32, #35, #36, #55, #56 | 40 days*

---

### Day 81 — Full-screen PDF viewer (#55)
**Phase 3 | Week 17**
- Upgrade `PageViewer` component: full-window view mode (toggle between embedded and full-screen)
- Fit-to-width default; support pinch-zoom on trackpad (wheel + Ctrl modifier)
- Smooth page transitions (CSS transition on page change)
- Status bar: page N of N, zoom %, file name
✅ Done when: PDF opens in full-screen mode with smooth navigation

### Day 82 — Overprint preview toggle
**Phase 3 | Week 17**
- Add "Overprint Preview" toggle button to viewer toolbar
- When on: render page via PDFium with overprint simulation flag (PDFium supports basic overprint simulation via render flags)
- Add visual indicator: "⚠ Overprint preview is approximate — use a RIP for production proof"
✅ Done when: toggle produces visually different rendering for a file with overprinting black

### Day 83 — Separation preview (plate view)
**Phase 3 | Week 17**
- "View Plate" button in viewer toolbar
- Dropdown: CMYK (C / M / Y / K plates), plus any spot colors found
- For CMYK plates: PDFium can render with a specific ink channel isolated (use FPDF_COLORSCHEME or render with color transform)
- Implementation: render normally, apply grayscale-per-channel shader in canvas 2D or CSS filter
✅ Done when: switching to "C plate" shows only cyan content in grayscale

### Day 84 — Color picker / inspector tool
**Phase 3 | Week 18**
- "Color Picker" tool in viewer toolbar (eyedropper icon)
- Click any point on the rendered page → return pixel color from rendered bitmap
- Show: RGB hex, CMYK approximate, LAB value
- Note: this reads rendered pixel values, not document color values — warn user of difference
✅ Done when: clicking on a known CMYK black area shows ~(0,0,0,100) CMYK values

### Day 85 — Measurement tool
**Phase 3 | Week 18**
- "Measure" tool: click two points on page → show distance in mm/inches/points
- Use CTM of page coordinate space to convert pixel distance to document units
- Show: distance, angle, horizontal component, vertical component
- Toggle measurement units in toolbar
✅ Done when: measuring a known 210mm A4 page width returns "210 mm"

### Day 86 — Layer management UI (#35)
**Phase 3 | Week 18**
- Write `commands::list_layers(path: String) -> Result<Vec<LayerInfo>, String>`
- `LayerInfo`: name, id, is_visible_default, is_locked, intent (View/Design/Export)
- Frontend: `LayerPanel` component — list of layers with eye icon toggle
✅ Done when: PDF with named layers (e.g., Illustrator layer export) shows layers by name

### Day 87 — Layer toggle + save
**Phase 3 | Week 18**
- Write `commands::set_layer_visibility(path: String, layer_id: String, visible: bool, output_path: String) -> Result<(), String>`
- Modifies OCG default state in `/OCProperties/D/ON` and `/OFF` arrays in document catalog
- Re-render current page after toggle to reflect change visually
✅ Done when: hiding a layer and re-rendering shows layer content removed

### Day 88 — Page operations (#36)
**Phase 3 | Week 19**
- Write `commands::extract_pages(path: String, page_indices: Vec<usize>, output_path: String) -> Result<(), String>`
- Write `commands::delete_pages(path: String, page_indices: Vec<usize>, output_path: String) -> Result<(), String>`
- Write `commands::rotate_page(path: String, page_index: usize, degrees: i32, output_path: String) -> Result<(), String>` (degrees: 90, 180, 270)
- Use lopdf to modify the Pages tree Kids array and page Rotate key
✅ Done when: deleting page 2 from a 5-page PDF produces a 4-page output file

### Day 89 — Page reorder + insert
**Phase 3 | Week 19**
- Write `commands::reorder_pages(path: String, new_order: Vec<usize>, output_path: String) -> Result<(), String>`
- Write `commands::insert_blank_page(path: String, after_index: usize, width_mm: f64, height_mm: f64, output_path: String) -> Result<(), String>`
- UI: drag-and-drop page reorder in thumbnail strip (using `@dnd-kit/core` or similar)
✅ Done when: drag-to-reorder in thumbnail strip produces correctly ordered output PDF

### Day 90 — Page operations UI
**Phase 3 | Week 19**
- Create `src/components/editing/PageOperationsPanel.tsx`:
  - Thumbnail grid with multi-select (Shift+click, Cmd+click)
  - Toolbar: Extract Selection / Delete Selection / Rotate / Insert Blank Page
  - Drag handles on thumbnails for reorder
- All operations write to a new output file with `_edited` suffix (never overwrite)
✅ Done when: multi-select + delete produces correct output; extracted pages are in new file

### Day 91 — Content stream round-trip
**Phase 3 | Week 19**
- Critical foundation for text + vector editing: content stream must round-trip cleanly
- Write `pdf::content_stream::decode_stream(doc: &Document, page_index: usize) -> Vec<u8>` — handle multiple content streams concatenated, handle FlateDecode, LZWDecode, DCTDecode filters
- Write `pdf::content_stream::encode_stream(tokens: Vec<Token>) -> Vec<u8>` — serialize tokens back to PDF operator syntax
- Round-trip test: decode → re-encode a page content stream → open result in PDFium → visually identical
✅ Done when: round-trip test passes for a complex page (text, images, paths, transparency)

### Day 92 — Text location via PDFium
**Phase 3 | Week 20**
- Use PDFium's text page API to get character positions on page
- Write `commands::get_text_objects(path: String, page_index: usize) -> Result<Vec<TextObject>, String>`
- `TextObject`: text (the string content), x, y, width, height (bounding box in page coordinates), font_name, font_size
- This is the "search" foundation — find all text regions and their positions
✅ Done when: a simple text-heavy page returns all text strings with bounding boxes

### Day 93 — Text search
**Phase 3 | Week 20**
- Write `commands::search_text(path: String, query: String) -> Result<Vec<TextMatch>, String>`
- `TextMatch`: page_index, text (full match), start_char, end_char, bounding_boxes
- Frontend: search bar in viewer — type query → jump to first match, highlight all matches on page
- Next/previous match navigation
✅ Done when: searching "Invoice" in a test PDF highlights all occurrences across pages

### Day 94 — Simple text replacement
**Phase 3 | Week 20**
- Scope: replace text where the replacement fits in the same approximate character width
- Strategy: find the `Tj`/`TJ` operator containing the target text in the content stream, replace string operand
- This works for: correcting typos, updating dates/numbers in same font/size
- Limitation: if replacement text is longer, it overflows the text box (warn user)
- Write `commands::replace_text(path: String, page_index: usize, find: String, replace: String, output_path: String) -> Result<ReplaceResult, String>`
- `ReplaceResult`: replacements_made, warnings (overflow, encoding issues)
✅ Done when: replacing "Jnauary" with "January" in a simple PDF works correctly

### Day 95 — Text replacement UI (#31)
**Phase 3 | Week 20**
- Create `src/components/editing/TextEditPanel.tsx`:
  - "Find & Replace" interface: find input, replace input, page scope (current / all)
  - "Find All" → shows list of matches with page number and context
  - "Replace" (current match) and "Replace All" buttons
  - Warning banner when replacement may overflow text box
  - Before/after preview (re-render affected page after replacement)
✅ Done when: end-to-end text replacement works from UI with visual confirmation

### Day 96 — Text edit edge cases
**Phase 3 | Week 20**
- Handle: text encoded as PDFDocEncoding, WinAnsiEncoding, MacRomanEncoding, UTF-16BE (each needs different string byte handling)
- Handle: text in Form XObjects (find → recurse into Form stream)
- Handle: text split across multiple `Tj` operators (adjacent operators rendering sequential chars)
- Warn for: text in Type3 fonts (glyph-as-content, can't be replaced by string substitution)
✅ Done when: text replacement works in a CJK PDF (UTF-16BE encoded) and a form XObject

### Day 97 — Image replacement (#32)
**Phase 3 | Week 21**
- Write `commands::replace_image(path: String, page_index: usize, xobject_name: String, new_image_path: String, output_path: String) -> Result<(), String>`:
  - Load new image via `image` crate (PNG, JPEG, TIFF supported)
  - Encode as JPEG DCT stream (or Flate for PNG lossless)
  - Update the XObject stream bytes, Width, Height, ColorSpace, BitsPerComponent, Filter keys
  - Keep all other XObject attributes (mask, decode array, etc.)
✅ Done when: replacing an image in a PDF produces a valid PDF with the new image

### Day 98 — Image selection UI
**Phase 3 | Week 21**
- "Select Image" tool in viewer toolbar: click image on page → highlight selected image
- Image selection requires finding which XObject's rendered bounding box contains the click point
- Show selected image info: name, pixel dimensions, DPI, color space, compression
- "Replace Image" button → file picker → run `replace_image`
✅ Done when: can click an image on page, see its info, and replace it with a new file

### Day 99 — Image editing operations (#32)
**Phase 3 | Week 21**
- Add these non-destructive image operations (applied during stream encode):
  - JPEG quality re-compress: re-encode JPEG at configurable quality
  - Downsample: resize to target DPI before embedding (using `image` crate `resize()`)
  - Convert grayscale: convert color image to grayscale
- Write `commands::optimize_image(path: String, xobject_name: String, settings: ImageOptimizeSettings, output_path: String) -> Result<(), String>`
✅ Done when: JPEG quality can be reduced from 95 to 70 for a selected image

### Day 100 — Image operations UI + Phase 3 PR
**Phase 3 | Week 21**
- Create `src/components/editing/ImageEditPanel.tsx`:
  - Image list (all images in document, or just current page)
  - Per-image: current DPI, size, compression — optimization controls
  - "Apply to All Low-DPI Images" batch action
- Run cargo check + tsc
- PR: "Phase 3 complete — PDF viewer and editing foundations"
- Merge
✅ Done when: PR merged; full editing workflow works end-to-end

---

## Phase 4 — Automation Engine
*Issues: #38, #39, #40, #41, #42 | 50 days*

---

### Day 101 — Preflight profile data model (#39)
**Phase 4 | Week 21**
- Add to DB:
  ```sql
  CREATE TABLE IF NOT EXISTS preflight_profiles (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
  );
  CREATE TABLE IF NOT EXISTS profile_checks (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES preflight_profiles(id) ON DELETE CASCADE,
    check_id TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'error',
    params TEXT NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL DEFAULT 0
  );
  CREATE TABLE IF NOT EXISTS profile_fixups (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES preflight_profiles(id) ON DELETE CASCADE,
    fixup_id TEXT NOT NULL,
    params TEXT NOT NULL DEFAULT '{}',
    condition_check_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
  );
  ```
- Write migrations for existing installs
✅ Done when: `cargo check` passes; tables created on next app launch

### Day 102 — Seed built-in profiles
**Phase 4 | Week 22**
- On first run, seed 4 built-in profiles:
  1. `PDF/X-1a`: all Phase 1+2 checks, X-1a color rules
  2. `PDF/X-4`: same but allow transparency
  3. `General Print (CMYK)`: warnings for RGB, no OutputIntent required
  4. `Wide Format (RGB OK)`: image DPI minimum 100, RGB allowed, no bleed required
- Mark all 4 with `is_builtin = 1` (cannot be deleted, only cloned)
- Write `commands::seed_builtin_profiles()` called at startup
✅ Done when: 4 profiles exist in DB after first launch

### Day 103 — Profile CRUD commands
**Phase 4 | Week 22**
- Write commands: `list_profiles`, `get_profile`, `create_profile`, `update_profile`, `delete_profile`
- `clone_profile(id, new_name)` → copies all checks and fixups to a new user profile
- Register all in `lib.rs`
✅ Done when: can create, clone, and delete a custom profile via Tauri devtools

### Day 104 — Check registry
**Phase 4 | Week 22**
- Define `CHECK_REGISTRY: &[CheckDefinition]` as a static list of all available checks:
  ```rust
  struct CheckDefinition {
      id: &'static str,        // "font_embedding", "image_dpi", etc.
      label: &'static str,
      description: &'static str,
      default_severity: Severity,
      params_schema: &'static str,  // JSON schema for params
  }
  ```
- Include all checks from Phase 1 + Phase 2
- Write `commands::list_check_definitions() -> Result<Vec<CheckDef>, String>`
✅ Done when: frontend can query the full list of available check types

### Day 105 — Profile editor UI
**Phase 4 | Week 22**
- Create `src/components/automation/ProfileEditor.tsx`:
  - Left panel: profile list (built-in marked as locked, user profiles editable)
  - Right panel: profile detail — name, description, list of checks (drag to reorder, enable/disable, edit params)
  - "Add Check" button → searchable check picker from registry
  - Each check row: check name, severity selector, params (rendered from JSON schema)
  - "Clone" and "Delete" on user profiles
✅ Done when: can create a custom profile with a subset of checks and custom min_dpi

### Day 106 — Profile params editing
**Phase 4 | Week 23**
- Render check params as form controls from JSON schema:
  - `type: "number"` → number input with min/max
  - `type: "string", enum: [...]` → select dropdown
  - `type: "boolean"` → checkbox
- Save params as JSON in `profile_checks.params` column
- Load params when running checks via that profile
✅ Done when: changing min_dpi in a profile to 150 causes image check to use 150 as threshold

### Day 107 — Fixup registry + profile fixups
**Phase 4 | Week 23**
- Define `FIXUP_REGISTRY` with all available fixups: `add_bleed`, `convert_rgb_to_cmyk`, `add_output_intent`, `replace_text` (future: more)
- Each fixup: id, label, description, params_schema, optional condition_check_id (apply only to objects failing a specific check)
- Add fixup editing to ProfileEditor: same UI pattern as checks but in "Fixups" tab
- Run order: fixups run before checks (fixes first, then verify)
✅ Done when: a profile can have fixups that auto-run before checks during full preflight

### Day 108 — Run profile via command
**Phase 4 | Week 23**
- Write `commands::run_profile(path: String, profile_id: i64, output_path: Option<String>) -> Result<ProfileRunResult, String>`:
  1. Load profile from DB
  2. Run fixups (in sort_order): call each fixup command in sequence, pass output of one as input to next
  3. Run checks (in sort_order) on the final (post-fixup) file
  4. Save findings to DB under new run record
  5. Return: findings, fixups applied, output file path
✅ Done when: running a profile with add_bleed fixup + bleed check produces fixed file that passes

### Day 109 — Action list data model (#38)
**Phase 4 | Week 23**
- Add to DB:
  ```sql
  CREATE TABLE IF NOT EXISTS action_lists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL
  );
  CREATE TABLE IF NOT EXISTS action_list_steps (
    id INTEGER PRIMARY KEY,
    action_list_id INTEGER NOT NULL REFERENCES action_lists(id) ON DELETE CASCADE,
    action_type TEXT NOT NULL,  -- "check" | "fixup"
    action_id TEXT NOT NULL,
    params TEXT NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
  );
  ```
✅ Done when: tables created; migration runs cleanly

### Day 110 — Action list record mode
**Phase 4 | Week 24**
- "Record" button in editing toolbar: starts recording session
- While recording: each command invoked (replace_text, add_bleed, convert_rgb_to_cmyk, etc.) is appended as an action_list_step
- "Stop Recording" button: saves recorded steps as named action list
- Show recording indicator (red dot) while recording is active
✅ Done when: performing add_bleed + convert_rgb_to_cmyk in sequence and stopping recording saves a 2-step action list

### Day 111 — Action list replay
**Phase 4 | Week 24**
- Write `commands::run_action_list(action_list_id: i64, path: String, output_path: String) -> Result<ActionListResult, String>`:
  - Load steps from DB
  - Execute each step in order on the file (pipe output of one step as input to next)
  - Return: steps executed, any step errors, output file path
- `ActionListResult`: `steps: Vec<StepResult>` where `StepResult` has success, message, output_size_bytes
✅ Done when: replay of a recorded action list produces same result as manual steps

### Day 112 — Action list UI
**Phase 4 | Week 24**
- Create `src/components/automation/ActionListPanel.tsx`:
  - List of saved action lists with step count
  - Click list → show steps (ordered list with action type, action name, params)
  - "Run on Current File" button: select output path, run, show result
  - "Edit Steps": drag-to-reorder, delete step, edit step params
  - "Record New" button
✅ Done when: full action list workflow (create via record, edit, replay) works from UI

### Day 113 — Batch processing data model (#40)
**Phase 4 | Week 24**
- Add to DB:
  ```sql
  CREATE TABLE IF NOT EXISTS batch_jobs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    input_folder TEXT NOT NULL,
    output_folder TEXT NOT NULL,
    profile_id INTEGER REFERENCES preflight_profiles(id),
    action_list_id INTEGER REFERENCES action_lists(id),
    status TEXT NOT NULL DEFAULT 'idle',
    created_at TEXT NOT NULL,
    last_run_at TEXT
  );
  CREATE TABLE IF NOT EXISTS batch_results (
    id INTEGER PRIMARY KEY,
    batch_job_id INTEGER NOT NULL REFERENCES batch_jobs(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    status TEXT NOT NULL,  -- "pass" | "fail" | "error"
    error_count INTEGER NOT NULL DEFAULT 0,
    warning_count INTEGER NOT NULL DEFAULT 0,
    output_file TEXT,
    processed_at TEXT NOT NULL
  );
  ```
✅ Done when: tables compile cleanly

### Day 114 — Batch processing command
**Phase 4 | Week 25**
- Write `commands::run_batch(batch_job_id: i64) -> Result<(), String>` (async, Tauri command)
- Logic:
  1. List all `.pdf` files in `input_folder`
  2. For each file: run profile or action list, save to `output_folder`
  3. Write `batch_results` row per file
  4. Emit progress events: `emit("batch_progress", { current: n, total: N, file_name, status })`
- Handle: file access errors, malformed PDFs, disk-full scenarios
✅ Done when: batch processes a folder of 10 PDFs and writes results rows

### Day 115 — Batch UI
**Phase 4 | Week 25**
- Create `src/components/automation/BatchPanel.tsx`:
  - Create batch job: name, input folder picker, output folder picker, select profile or action list
  - "Run Batch" button
  - Live progress bar + current file name during processing
  - Results table: file name | status badge | error count | warning count | output file link
  - "Export Summary CSV" button
✅ Done when: batch processing a folder from UI shows live progress and final results table

### Day 116 — Batch fail routing
**Phase 4 | Week 25**
- Add optional "Fail Folder" to batch job config
- Files that fail preflight: copy (or move, configurable) to fail folder
- Files that pass: copy to output (pass) folder
- This is the basic hot-folder routing model
✅ Done when: running batch on mixed pass/fail PDFs correctly routes them to separate output folders

### Day 117 — Batch report generation
**Phase 4 | Week 25**
- After batch completes: auto-generate a summary report PDF using `printpdf`
- Report: job name, date/time, total files, pass count, fail count, per-file status table
- Save report to output folder as `batch_report_[timestamp].pdf`
✅ Done when: batch report PDF opens correctly and shows accurate summary

### Day 118 — Action list debugger UI skeleton (#41)
**Phase 4 | Week 26**
- Create `src/components/automation/ActionListDebugger.tsx`:
  - Load an action list and a target PDF
  - "Step Forward" button: run one step at a time
  - Show current step highlighted in step list
  - Show step result: success/failure, time taken, output
  - Re-render current page after each step
✅ Done when: stepping through a 3-step action list advances one step at a time

### Day 119 — Debugger: before/after view
**Phase 4 | Week 26**
- After each step: show split before/after render of the current page
- Left panel: page state before this step
- Right panel: page state after this step
- For checks: show finding list (what the check found on this document state)
- For fixups: show what changed (finding list + visual diff)
✅ Done when: applying add_bleed fixup shows page boxes changed in before/after render

### Day 120 — Debugger: finding detail
**Phase 4 | Week 26**
- Expand each step result: show all findings produced by that step
- Click finding → jump to relevant page, highlight affected object if possible
- "Run from Here" button: restart from a specific step with the current document state
✅ Done when: can navigate between findings and jump to affected pages from debugger

### Day 121 — Debugger: save debug session
**Phase 4 | Week 26**
- Save debug session state to DB: which action list, which PDF, current step index, per-step results
- "Reopen" debug session on next launch for the same action list + PDF combination
- This lets you close the app and return to a debug session without re-running everything
✅ Done when: closing and reopening the debugger resumes at the last step

### Day 122 — Debugger: export debug report
**Phase 4 | Week 26**
- "Export Debug Report" button: generates PDF with:
  - Action list steps
  - Per-step: success/failure, time, finding count
  - Screenshots of page before/after each step
- Useful for sharing debugging results with team
✅ Done when: exported debug report PDF opens correctly

### Day 123 — Hot folder watcher: platform setup (#42)
**Phase 4 | Week 27**
- Add `notify = { version = "7", features = ["serde"] }` to Cargo.toml
- Write `pdf::watcher::FolderWatcher` struct:
  - `start(path: PathBuf, tx: Sender<WatchEvent>)` using `notify::RecommendedWatcher`
  - Debounce rapid events (write → close → written sequence = one event) using `notify-debouncer-full`
  - Filter: only trigger on `.pdf` files; ignore temp files (`.tmp`, `~$`, `.crdownload`)
✅ Done when: watcher fires events when a PDF is added to a watched folder

### Day 124 — Hot folder config + DB
**Phase 4 | Week 27**
- Add `hot_folders` table:
  ```sql
  CREATE TABLE IF NOT EXISTS hot_folders (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    watch_path TEXT NOT NULL,
    pass_path TEXT NOT NULL,
    fail_path TEXT NOT NULL,
    profile_id INTEGER REFERENCES preflight_profiles(id),
    action_list_id INTEGER REFERENCES action_lists(id),
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
  )
  ```
- Write CRUD commands for hot_folders
✅ Done when: hot folder config can be created and persisted

### Day 125 — Hot folder processing loop
**Phase 4 | Week 27**
- Write `commands::start_hot_folder_service()` — starts all active hot folders
- Called at app startup (add to `setup` in `lib.rs`)
- For each new PDF detected: run the configured profile + route to pass/fail folder
- Log each processing event to a new `hot_folder_log` table
- Emit `hot_folder_event` Tauri event to update UI in real-time
✅ Done when: dropping a PDF into a watched folder automatically processes it and routes it

### Day 126 — Hot folder UI
**Phase 4 | Week 27**
- Create `src/components/automation/HotFolderPanel.tsx`:
  - Hot folder list: name | watch path | active toggle | last activity
  - Add/edit/delete hot folders
  - Live activity log: scrolling list of recently processed files with status
  - "Open Watch Folder" and "Open Pass/Fail Folders" quick links
✅ Done when: hot folder management and live activity log work from the UI

### Day 127 — Hot folder error recovery
**Phase 4 | Week 28**
- Files that error (corrupt, permission denied): move to a `_error` subfolder under fail path
- Retry logic: if file is still being written (file size changing), wait and retry up to 3 times
- Alert: emit `notification` event when a file fails processing — show system notification via Tauri notification plugin
✅ Done when: a corrupted PDF in the watch folder is moved to _error folder; desktop notification appears

### Day 128 — Hot folder performance
**Phase 4 | Week 28**
- Support configurable max concurrent processing (default 2) — don't process 20 files simultaneously on old hardware
- Add processing queue: files arrive → queue → process up to N at once
- Show queue depth in hot folder UI
✅ Done when: dropping 10 PDFs at once processes them 2 at a time with queue display

### Day 129 — Phase 4 automation integration test
**Phase 4 | Week 28**
- Full workflow test:
  1. Create a profile with all Phase 1+2 checks
  2. Record an action list: add_bleed → convert_rgb_to_cmyk
  3. Run action list via batch on a folder of 5 test PDFs
  4. Verify output files are correct
  5. Enable hot folder, drop a PDF, verify auto-processing
  6. Debug the action list using the debugger
✅ Done when: all 4 automation features work together without errors

### Day 130 — Phase 4 PR + merge
**Phase 4 | Week 28**
- cargo check, tsc --noEmit, fix all issues
- PR: "Phase 4 complete — automation engine (profiles, action lists, batch, hot folders, debugger)"
- Merge to main
✅ Done when: clean merge; automation features available in main branch

---

## Phase 5 — Advanced Features
*Issues: #45, #48, #49, #50, #59, #60 | 50 days*

---

### Day 131 — PDF compression architecture (#49)
**Phase 5 | Week 29**
- Add `image = "0.25"` to Cargo.toml
- Write `pdf::compress::CompressionSettings` struct: target_image_dpi, jpeg_quality (0-100), compress_streams (Flate), subset_fonts (Phase 5.5), remove_metadata (bool), remove_unused_objects (bool)
- Write `commands::analyze_compression_potential(path: String) -> Result<CompressionAnalysis, String>`:
  - Count images, measure total image stream bytes
  - Identify images above target DPI (potential downsamples)
  - Estimate compressed size
✅ Done when: analysis command returns accurate current file size and estimated savings

### Day 132 — Image downsampling
**Phase 5 | Week 29**
- Write `pdf::compress::downsample_image(img_data: &[u8], color_space: &str, width: u32, height: u32, current_dpi: f64, target_dpi: f64) -> (Vec<u8>, u32, u32)`:
  - Decode raw image stream bytes to `image::DynamicImage`
  - Resize to new pixel dimensions maintaining aspect ratio
  - Re-encode as JPEG at configured quality
- Apply to all images above threshold in `compress_pdf` command
✅ Done when: a 300 DPI image is correctly downsampled to 150 DPI in output PDF

### Day 133 — Stream re-compression
**Phase 5 | Week 29**
- Re-compress uncompressed or LZW-encoded streams with Flate (zlib/deflate):
  - Non-image streams: content streams, XMP metadata, etc.
  - Add `miniz_oxide = "0.8"` to Cargo.toml for Deflate compression
- Remove unused objects: scan for unreferenced objects in cross-reference table, delete before saving
✅ Done when: PDF with uncompressed streams is smaller after compression

### Day 134 — `compress_pdf` command
**Phase 5 | Week 30**
- Write `commands::compress_pdf(path: String, settings: CompressionSettings, output_path: String) -> Result<CompressionResult, String>`
- `CompressionResult`: original_size_bytes, compressed_size_bytes, reduction_percent, images_downsampled, streams_recompressed
- Register in `lib.rs`
✅ Done when: typical print PDF (30MB) compresses to < 10MB with standard settings

### Day 135 — Compression UI
**Phase 5 | Week 30**
- Create `src/components/advanced/CompressionPanel.tsx`:
  - Settings form: target image DPI (slider), JPEG quality (slider), compress streams (toggle)
  - "Analyze" button: shows estimated savings before committing
  - Progress bar during compression
  - Result: before/after size comparison with reduction percentage
✅ Done when: full compression workflow works from UI with size display

### Day 136 — XMP metadata cleanup
**Phase 5 | Week 30**
- Option to strip XMP metadata streams (useful for reducing size when metadata is not needed)
- Option to strip document info dictionary fields (author, creator, producer) for privacy
- Preserve required keys for PDF/X: OutputIntents, GTS_PDFXVersion
- Write `commands::strip_metadata(path: String, output_path: String) -> Result<(), String>`
✅ Done when: stripped PDF is smaller and no longer shows Creator/Producer in Acrobat

### Day 137 — Font subset validation
**Phase 5 | Week 30**
- Use `ttf-parser` to parse embedded font streams:
  - Check `cmap` table for character-to-glyph mapping
  - Check which glyphs are actually referenced vs. which are in the font data
  - Calculate: percentage of glyphs embedded that are actually used
- Report: "Font 'Arial' has 1,245 glyphs embedded but only 23 are used on this page — 98% waste"
✅ Done when: report correctly identifies over-embedded fonts

### Day 138 — Font subsetting (write)
**Phase 5 | Week 31**
- This is the hardest part of compression: actually removing unused glyphs from embedded font streams
- Write `pdf::compress::subset_font(font_stream: &[u8], used_glyph_ids: &[u16]) -> Vec<u8>`:
  - Parse TrueType tables using `ttf-parser`
  - Build new subset: keep only used glyph IDs in `glyf`, `loca`, `hmtx` tables
  - Rebuild `head`, `hhea`, `maxp` with updated counts
  - Re-encode as TrueType stream
- This is complex — allocate 3 days (Days 138–140)
✅ Done when: font stream bytes are smaller; subsetted font still renders correctly in Acrobat

### Day 139 — Font subsetting: cmap and name tables
**Phase 5 | Week 31**
- Rebuild `cmap` table to only reference used glyph IDs
- Update `name` table: add `ABCDEF+` prefix to PostScript name to indicate subset
- Handle: composite fonts (CIDFont Type2) require also updating the ToUnicode CMap stream
- Handle: TrueType hinting (fpgm, prep tables should be preserved)
✅ Done when: subsetted composite font (CJK) renders correctly in Acrobat

### Day 140 — Font subsetting integration + test
**Phase 5 | Week 31**
- Integrate font subsetting into `compress_pdf` pipeline (runs after image compression)
- Handle edge cases: OpenType CFF fonts (different table structure — skip subsetting, flag as warning)
- Test: compress a 10MB PDF with large embedded fonts → should reduce to < 3MB
✅ Done when: font subsetting produces measurable file size reduction on a real document

### Day 141 — Barcode detection setup (#48)
**Phase 5 | Week 32**
- Research `zxingcpp-rs` crate (Rust bindings to ZXing-C++ v2, Apache 2.0) — verify Windows + macOS support
- Add to Cargo.toml; compile test
- Write `commands::detect_barcodes(path: String, page_index: usize) -> Result<Vec<BarcodeDetection>, String>`:
  - Render page to bitmap at 200 DPI via PDFium
  - Pass bitmap to ZXing for barcode detection
  - Return: format, decoded text, position, orientation
✅ Done when: ZXing detects a Code 128 barcode from a rendered page bitmap

### Day 142 — Barcode quiet zone validation
**Phase 5 | Week 32**
- For each detected barcode: check quiet zone (white space around it)
- Quiet zone requirements: Code 128 = 10× narrow bar width; QR = 4× module size; EAN-13 = 3.63mm minimum
- Measure: expand barcode bounding box, check for non-white pixels in quiet zone area
- `BarcodeValidation`: format, decoded, position, quiet_zone_left_ok, quiet_zone_right_ok, quiet_zone_top_ok, quiet_zone_bottom_ok, severity
✅ Done when: barcode with insufficient quiet zone is flagged with which sides are tight

### Day 143 — Barcode size and decodability check
**Phase 5 | Week 32**
- Size check: compare rendered barcode dimensions against minimum size specs:
  - EAN-13: minimum 26.73mm wide × 18.28mm tall (80% magnification)
  - Code 128: minimum 19mm tall (non-retail)
  - QR: minimum 10mm × 10mm at 300 DPI
- Decodability: verify ZXing can decode it cleanly (if it was detected, it decoded — log content)
- Show decoded content: user can verify barcode encodes the right data
✅ Done when: undersized barcode is flagged with minimum size requirement message

### Day 144 — BarcodeCheck UI
**Phase 5 | Week 32**
- Create `src/components/advanced/BarcodeCheck.tsx`:
  - Run check button: detect all barcodes on all pages
  - Table: page | format | decoded content | quiet zone | size | status
  - Click barcode → highlight its location on page render (overlay rectangle)
✅ Done when: barcode table shows all detected barcodes with validation status

### Day 145 — Barcode check integration into preflight
**Phase 5 | Week 33**
- Add barcode check to the General Print profile as an optional check (off by default)
- Add barcode check definition to CHECK_REGISTRY
- Profile editor: enable barcode check → add to profile → barcode findings appear in preflight report
✅ Done when: enabling barcode check in a profile causes barcode findings to appear in run_profile results

### Day 146 — Analytics dashboard data (#50)
**Phase 5 | Week 33**
- Write SQL aggregation queries for existing `preflight_findings` and `batch_results` data:
  - Pass rate by date (daily for last 30 days)
  - Most common errors by check_name (top 10)
  - Average error count per file by client (if client_id associated)
  - Files processed per day (batch + manual)
  - Average file size trend
- Write `commands::get_analytics(days: i64) -> Result<AnalyticsSummary, String>`
✅ Done when: query returns accurate aggregates matching manual count of test data

### Day 147 — Analytics UI
**Phase 5 | Week 33**
- Add `recharts` to package.json (`npm install recharts`)
- Create `src/components/advanced/AnalyticsDashboard.tsx`:
  - Pass rate over time: line chart (last 30 days)
  - Most common errors: horizontal bar chart (top 10 errors by occurrence)
  - Files processed: bar chart per day
  - Summary cards: total files this month, pass rate this month, most common error
✅ Done when: analytics dashboard renders with real data from DB

### Day 148 — Analytics per-client view
**Phase 5 | Week 33**
- Link `pdf_jobs` to `clients` table (add optional `client_id` column to `pdf_jobs`)
- Prompt "Associate with client?" when opening a PDF (optional)
- Per-client analytics: pass rate, most common errors, average turnaround
- In `AnalyticsDashboard`: client filter dropdown
✅ Done when: opening a PDF and tagging it to a client causes client-filtered stats to update

### Day 149 — Analytics export
**Phase 5 | Week 34**
- "Export Analytics Report" button → generates PDF report with charts (use `printpdf` for layout, save chart as PNG via canvas)
- "Export Raw Data CSV" → downloads `preflight_findings` as CSV
- Date range filter on analytics: this month / last 30 days / last 90 days / custom range
✅ Done when: exported analytics PDF opens correctly with charts and summary data

### Day 150 — Approval sheet generation (#60)
**Phase 5 | Week 34**
- Add `printpdf = "0.9"` to Cargo.toml (already planned; add if not present)
- Write `commands::generate_approval_sheet(path: String, job_info: ApprovalSheetInfo, output_path: String) -> Result<(), String>`
- `ApprovalSheetInfo`: client_name, job_number, due_date, description, staff_name, include_preflight_summary
- Layout (A4 portrait):
  - Header: shop logo (if configured), job number, date
  - Page thumbnails grid (2×3 per sheet, 150 DPI)
  - Job info table
  - Preflight summary (error/warning counts) if requested
  - Sign-off lines: "Approved by: ________ Date: ________"
✅ Done when: approval sheet PDF generates with thumbnails and job info

### Day 151 — Approval sheet customization
**Phase 5 | Week 34**
- Load shop logo from `business_info` (if a logo path is configured, render it)
- Configurable colors (from business brand color)
- Option: include full preflight findings list (expands to multiple pages if needed)
- Option: add watermark "PROOF" in diagonal across each page thumbnail
✅ Done when: approval sheet with logo and PROOF watermark looks professional

### Day 152 — Approval sheet from ArtApprovalPanel
**Phase 5 | Week 34**
- Wire approval sheet generation into `ArtApprovalPanel`:
  - "Generate Approval Sheet" button on each approval version
  - Auto-fills job number from order, client name from linked client
  - Opens save file dialog for output path
✅ Done when: can generate an approval sheet directly from an order's art approval panel

### Day 153 — Report export: PDF (#59)
**Phase 5 | Week 35**
- Write `commands::export_preflight_report_pdf(job_id: i64, output_path: String) -> Result<(), String>`
- Layout: shop header, file info, page count, run date, findings table (finding | page | severity | message)
- Group findings by category (Font / Page Boxes / Bleed / Color / PDF/X Compliance)
- Pass/fail summary on first page
✅ Done when: exported report PDF matches the on-screen report layout

### Day 154 — Report export: CSV and JSON (#59)
**Phase 5 | Week 35**
- Write `commands::export_preflight_report_csv(job_id: i64, output_path: String) -> Result<(), String>`
  - Columns: check_name, severity, page_num, message, fix_hint
- Write `commands::export_preflight_report_json(job_id: i64) -> Result<String, String>`
  - Returns full structured JSON — useful for MIS integration webhook
- Add "Export Report" dropdown to PreflightReport UI: PDF / CSV / JSON
✅ Done when: all three export formats download correctly

### Day 155 — AI visual checking setup (#45)
**Phase 5 | Week 35**
- Settings panel: API key configuration for AI provider (Anthropic Claude API preferred — best vision model)
- Write `commands::check_ai_visual(path: String, page_index: usize, api_key: String) -> Result<AiVisualFinding, String>`:
  - Render page at 150 DPI
  - Encode as base64 PNG
  - Send to Claude API with system prompt: "You are a print preflight expert. Examine this page for visual quality issues: blur, pixelation, illegible text, unintended white areas, color banding, misaligned elements. List specific issues found. If none, say PASS."
  - Parse response into structured finding
✅ Done when: sending a deliberately blurry test page to the API returns a finding about image quality

### Day 156 — AI visual check batching
**Phase 5 | Week 36**
- "AI Visual Check" button in preflight report: runs AI check on all pages (with rate limiting)
- Progress: "Checking page N of N with AI..."
- Rate limit: max 10 pages/minute to avoid API cost spikes
- Per-page findings: show AI's assessment alongside rules-based findings
- Cost estimate: show estimated token usage before running (approximately 1,000 tokens per page at 150 DPI)
✅ Done when: AI check runs on a 5-page PDF and returns useful findings for each page

### Day 157 — AI visual finding UI
**Phase 5 | Week 36**
- Integrate AI findings into PreflightReport as an "AI Visual Review" section
- Each AI finding: page number, issue description, severity (AI's assessment), quoted text from AI response
- "Recheck this page" button for re-running on a single page
- Clear disclaimer: "AI checks are supplementary. Always verify critical issues manually."
✅ Done when: AI findings appear in the combined report with clear visual distinction

### Day 158 — AI visual check fine-tuning
**Phase 5 | Week 36**
- Refine the system prompt based on real test results:
  - Too many false positives → add "Only report issues that would affect print quality"
  - Missing issues → add specific categories: "Check for: very small text (< 6pt), text touching the edge..."
- Add user-configurable prompt prefix for shop-specific checks (e.g., "This shop prints on uncoated stock — flag any very light tints below 10%")
✅ Done when: AI check produces < 2 false positives on a known-good PDF

### Day 159 — Ink coverage check (#26 + future)
**Phase 5 | Week 36**
- Implement rendering-based ink coverage estimate:
  - Render page at 72 DPI with PDFium to CMYK-approximate bitmap
  - Sample all pixels, calculate: average C+M+Y+K per pixel
  - Flag pages where average TAC exceeds threshold (typical: 300% for coated, 260% for uncoated)
  - This is an estimate — note it doesn't account for overprint correctly
- Write `commands::check_ink_coverage(path: String, max_tac: f64) -> Result<Vec<InkCoverageFinding>, String>`
✅ Done when: a known rich-black heavy PDF reports TAC near 400% (0+0+0+100 black = 100%, rich black = 300-400%)

### Day 160 — Phase 5 integration + PR
**Phase 5 | Week 36**
- Add all new Phase 5 features to ManagementView PDF section navigation
- Run full preflight + AI check + compression + barcode check on 5 real PDFs
- cargo check + tsc --noEmit
- PR: "Phase 5 complete — compression, barcodes, AI visual, analytics, reports"
- Merge
✅ Done when: Phase 5 PR merged cleanly

---

## Phase 6 — Integration and Polish
*Issues: #54, #57, #58 + MIS webhook + final QA | 40 days*

---

### Day 161 — Email integration setup (#54)
**Phase 6 | Week 37**
- Add `lettre = { version = "0.11", features = ["tokio1-native-tls"] }` to Cargo.toml
- Add SMTP settings to business settings UI: host, port, username, password, use_tls, from_address
- Store settings in `business_info` table (add columns)
- Write `commands::test_smtp_connection() -> Result<(), String>` — sends a test email
✅ Done when: valid SMTP credentials allow sending a test email from the app

### Day 162 — Email preflight report
**Phase 6 | Week 37**
- Write `commands::email_preflight_report(job_id: i64, to_address: String, subject: String, body: String) -> Result<(), String>`:
  - Generate the preflight report PDF to a temp file
  - Build email: from settings, to address, subject, body, attachment
  - Send via lettre SMTP
- Frontend: "Email Report" button in PreflightReport → modal with to/subject/body fields
✅ Done when: preflight report arrives in inbox as PDF attachment

### Day 163 — Email approval sheet
**Phase 6 | Week 37**
- "Send for Approval" button in ArtApprovalPanel:
  - Generate approval sheet PDF
  - Email to client's email address (from linked client record)
  - Pre-fill subject: "Proof for Approval — [Order Number]"
  - Pre-fill body: customizable template from business settings
- Log email as an invoice reminder (method = email) in the existing `invoice_reminders` table
✅ Done when: approval sheet emails correctly to client address from ArtApprovalPanel

### Day 164 — FTP output routing (#54)
**Phase 6 | Week 38**
- Add `suppaftp = "5"` to Cargo.toml (pure Rust FTP client, MIT license)
- Add FTP settings to business settings: host, port, username, password, base_path, use_ftps
- Write `commands::test_ftp_connection() -> Result<(), String>`
- Write `commands::upload_to_ftp(local_path: String, remote_path: String) -> Result<(), String>`
✅ Done when: can upload a file to an FTP server from the app

### Day 165 — FTP integration with hot folders
**Phase 6 | Week 38**
- Add "FTP Upload" as an output action in hot folder config:
  - Pass files → upload to `base_path/pass/` on FTP
  - Fail files → upload to `base_path/fail/`
- Add FTP upload status to hot folder activity log
✅ Done when: hot folder processing auto-uploads processed PDFs to FTP server

### Day 166 — Email/FTP test with real servers
**Phase 6 | Week 38**
- Test SMTP with Gmail, Outlook, and a generic SMTP server (Mailgun/Sendgrid)
- Test FTP with FileZilla Server locally and a remote FTP server
- Fix connection issues, TLS certificate handling, passive/active mode
✅ Done when: all tested SMTP and FTP configurations connect without error

### Day 167 — MIS webhook output
**Phase 6 | Week 38**
- Add webhook settings to business settings: webhook URL, auth header (optional bearer token)
- Write `commands::send_webhook(job_id: i64, event_type: String) -> Result<(), String>`:
  - Build JSON payload: job info, file name, preflight result summary, per-check results
  - POST to configured URL with auth header
  - Log webhook delivery status and HTTP response code
- Events: `preflight_complete`, `batch_complete`, `hot_folder_processed`
✅ Done when: preflight completing sends a POST to a local webhook.site receiver with correct JSON

### Day 168 — Webhook retry logic
**Phase 6 | Week 39**
- Retry failed webhooks: up to 3 retries with exponential backoff (1s, 4s, 16s)
- Store pending webhooks in DB: `webhook_deliveries` table with status (pending/sent/failed)
- Background task: retry pending webhooks on app startup and after each new delivery attempt
✅ Done when: webhook to an offline server retries and succeeds when server comes back

### Day 169 — Keyboard shortcuts (#58)
**Phase 6 | Week 39**
- Implement global keyboard shortcuts in React using `useEffect` + `keydown` listener:
  - `Cmd/Ctrl+O`: open PDF file picker
  - `Cmd/Ctrl+R`: run preflight with current profile
  - `Cmd/Ctrl+B`: open batch panel
  - Arrow Left / Arrow Right: previous/next page in viewer
  - `+` / `-` or `Cmd/Ctrl+=` / `Cmd/Ctrl+-`: zoom in/out
  - `Cmd/Ctrl+Shift+E`: export report
  - `Cmd/Ctrl+Shift+F`: find & replace
  - `Escape`: close modal / deselect tool
  - `?`: open keyboard shortcut reference
- Register shortcuts only when PDF section is active (not when order forms have focus)
✅ Done when: all shortcuts work without conflicting with text inputs

### Day 170 — Keyboard shortcut reference UI
**Phase 6 | Week 39**
- Shortcut overlay: pressing `?` shows a modal with all shortcuts grouped by category
- Each shortcut: key combo | description | section it applies in
- Keyboard shortcut hints on all buttons (tooltip shows shortcut if available)
✅ Done when: shortcut reference overlay is complete and accurate

### Day 171 — Help system architecture (#57)
**Phase 6 | Week 39**
- Create `src-tauri/resources/help/` directory with markdown files:
  - `overview.md`, `font-embedding.md`, `image-dpi.md`, `color-spaces.md`, `bleed.md`, `pdfx.md`, `batch.md`, `hot-folders.md`
- Write `commands::get_help_article(slug: String) -> Result<String, String>` — reads markdown from resources
- Add `pulldown-cmark` or similar to parse markdown in frontend: `npm install marked`
✅ Done when: a help article loads as rendered HTML from the resources folder

### Day 172 — Help panel UI
**Phase 6 | Week 39**
- Create `HelpPanel` component (slide-out from right, `?` button in top nav):
  - Search: filter articles by keyword
  - Article list: grouped by category (Preflight / Color / Automation / Integration)
  - Article view: rendered markdown with syntax highlighting for code blocks
✅ Done when: help panel slides in and shows searchable articles

### Day 173 — Contextual help links
**Phase 6 | Week 40**
- Add "?" help icon next to every check in PreflightReport
- Clicking "?" opens HelpPanel with the article relevant to that check
- Map: check_id → help article slug
- Also add "Learn more" link in fix hints
✅ Done when: clicking "?" next to "Font embedding error" opens the font embedding help article

### Day 174 — Help article authoring: font checks
**Phase 6 | Week 40**
- Write complete help articles for all font-related checks:
  - What font embedding is and why it matters
  - How to embed fonts in InDesign (step by step with settings)
  - How to embed fonts in Illustrator
  - How to embed fonts via Acrobat Pro
  - What font subsetting means and when to use it
✅ Done when: font embedding article is clear enough for a non-technical print shop employee

### Day 175 — Help articles: color and bleed
**Phase 6 | Week 40**
- Write help articles:
  - Color spaces: CMYK vs RGB, when each is used, why print needs CMYK
  - PDF/X-1a: what it is, who requires it, what it guarantees
  - Bleed: what bleed is, why it's needed, how to add it in design software
  - Overprint: what it means, when it's correct, when it's an error
✅ Done when: all 4 articles are complete and accurate

### Day 176 — Help articles: automation features
**Phase 6 | Week 40**
- Write help articles:
  - Preflight profiles: how to create and customize them
  - Action lists: recording and replaying automation steps
  - Batch processing: setting up and running batch jobs
  - Hot folders: configuring automatic PDF processing
✅ Done when: all automation help articles are written

### Day 177 — Onboarding improvements
**Phase 6 | Week 41**
- First-time PDF Tools visit: show a brief "Getting Started" overlay
  - Step 1: Open a PDF
  - Step 2: Choose a preflight profile
  - Step 3: Run preflight and view results
- "Skip" and "Don't show again" buttons
- Persist "has_seen_pdf_onboarding" in `business_info`
✅ Done when: first launch shows onboarding overlay; subsequent launches skip it

### Day 178 — Performance audit
**Phase 6 | Week 41**
- Profile the slowest commands on a 200-page, 100MB PDF:
  - Thumbnail generation (first 20 pages)
  - Full preflight run
  - Batch processing 20 files
- Add `tokio::time::Instant` timing to each major command, log to console
- Target: preflight on a 50-page PDF in < 10 seconds on a 10-year-old CPU (Core i5 6th gen)
✅ Done when: performance targets met or specific bottlenecks documented with plans to address

### Day 179 — Memory usage audit
**Phase 6 | Week 41**
- Test memory usage: open 5 large PDFs in sequence
- Verify temp render files are cleaned up after page navigation
- Add `cleanup_temp_files()` called on app close and when opening a new PDF
- Verify PDFium document handles are released when a PDF is closed
✅ Done when: app memory stays below 500MB after processing 10 large PDFs in sequence

### Day 180 — Cross-platform test: macOS
**Phase 6 | Week 41**
- Test all Phase 1–6 features on macOS Apple Silicon
- PDFium binary: verify ARM64 binary loads correctly
- File paths: macOS uses `/Users/...`; verify path handling is cross-platform
- Hot folders: FSEvents on macOS — verify notify works correctly
- Fix any macOS-specific issues
✅ Done when: all features work on macOS without modification

### Day 181 — Cross-platform test: Windows older hardware
**Phase 6 | Week 42**
- Test on Windows 10 / older CPU (Core i5 4th gen or equivalent)
- Verify PDFium loads on Windows 10 (not just Windows 11)
- Verify file dialog works for PDF filter
- Verify hot folder watcher works on Windows with NTFS
✅ Done when: core preflight and batch features work on older Windows hardware

### Day 182 — Accessibility pass
**Phase 6 | Week 42**
- Run through PreflightReport, ProfileEditor, BatchPanel with keyboard only (no mouse)
- All interactive elements reachable via Tab key
- All status colors also differentiated by icon (not color alone)
- ARIA labels on icon-only buttons
✅ Done when: full preflight workflow completable without a mouse

### Day 183 — Error message audit
**Phase 6 | Week 42**
- Audit all `Err(...)` returns in `commands.rs` for PDF-related commands
- Every error message should:
  - State what failed (not "Error" but "Failed to load PDF: file is encrypted")
  - Suggest an action (not "Error" but "Open an unencrypted copy or enter the password")
- Add inline banners in PDF view for common recoverable errors
✅ Done when: no error message says just "Error" or a Rust debug string

### Day 184 — Settings panel for PDF tools
**Phase 6 | Week 42**
- Create `src/components/settings/PdfSettings.tsx` section in main settings:
  - Default preflight profile selector
  - Default output directory (for fixup outputs)
  - SMTP email settings (test button)
  - FTP settings (test button)
  - Webhook URL + auth header
  - AI API key (with token usage display)
  - Temp file cleanup interval
✅ Done when: all settings save correctly and persist across app restarts

### Day 185 — Integration test: full workflow
**Phase 6 | Week 43**
- End-to-end test with a real print shop scenario:
  1. Client submits a PDF (RGB, no bleed, unembedded font)
  2. Open PDF in app, run PDF/X-1a preflight → 3 errors
  3. Run "Make PDF/X-1a" wizard: add bleed, convert colors, flag font issue
  4. Re-run preflight → 1 error (font, can't auto-fix)
  5. Send approval sheet to client email
  6. Generate preflight report PDF and save
  7. Mark QB sync status on linked invoice
✅ Done when: entire workflow completes without errors or confusion

### Day 186 — Integration test: batch + hot folder
**Phase 6 | Week 43**
- Set up hot folder pointing to a test directory
- Drop 10 PDFs (mix of pass/fail) into folder
- Verify: all processed automatically, routed to pass/fail folders, FTP upload fires, webhook fires
- Check batch report PDF is generated in output folder
✅ Done when: automated batch processing works without manual intervention

### Day 187 — Bug bash day 1
**Phase 6 | Week 43**
- Structured bug hunt: go through every UI component in PDF Tools section
- For each component: try every button, every edge case input, every error state
- File GitHub issues for anything found
- Fix P0 (crash) and P1 (data loss) bugs immediately
✅ Done when: no P0 or P1 bugs remain; P2/P3 documented

### Day 188 — Bug bash day 2
**Phase 6 | Week 43**
- Continue bug bash: automation features (profiles, action lists, batch, hot folders)
- Specifically test: concurrent batch + hot folder running simultaneously
- Test: action list with a step that fails halfway — verify partial output is handled gracefully
✅ Done when: no new P0/P1 bugs found

### Day 189 — Documentation: release notes
**Phase 6 | Week 44**
- Write user-facing feature description for each new feature (not technical documentation)
- Format: what it does, how to use it, common use cases
- This doubles as in-app onboarding copy and marketing copy
✅ Done when: all major PDF tooling features have a user-facing description

### Day 190 — Final cargo check + tsc pass
**Phase 6 | Week 44**
- `cargo check --manifest-path src-tauri/Cargo.toml` — zero warnings
- `npx tsc --noEmit` — zero errors
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — fix all clippy warnings
✅ Done when: clean build with no warnings in either Rust or TypeScript

### Day 191 — Final PR: Phase 6
**Phase 6 | Week 44**
- Commit: email integration, FTP integration, keyboard shortcuts, help system, settings
- PR: "Phase 6 complete — integration and polish"
- Merge to main
✅ Done when: all 6 phases merged to main

### Day 192 — Regression test: Phase 1 checks still accurate
**Phase 6 | Week 44**
- Re-run the 10 test PDFs from Day 50 with the final app
- Verify Phase 1 results haven't changed despite 140+ days of additional code
- Fix any regressions introduced during later phases
✅ Done when: Phase 1 results match Day 50 baseline on all 10 test files

### Day 193 — Performance final baseline
**Phase 6 | Week 44**
- Run full benchmark suite on target hardware (6th gen Core i5, 8GB RAM):
  - Open a PDF: < 2 seconds
  - Thumbnail strip (20 pages): < 5 seconds
  - Full preflight (50 pages): < 10 seconds
  - Batch (20 files): throughput documented
- Record these as the official performance baseline
✅ Done when: benchmarks documented and all targets met

### Day 194 — Post-launch issue triage plan
**Phase 6 | Week 45**
- Create GitHub issue templates for: bug report, feature request, preflight discrepancy (our result vs. Acrobat)
- Preflight discrepancy template: include PDF/X version checked, our finding, what Acrobat reports, PDF upload path
- Write triage guidelines: who handles which issue type, response time targets
✅ Done when: issue templates are in `.github/ISSUE_TEMPLATE/`

### Day 195 — v1.0 release preparation
**Phase 6 | Week 45**
- `cargo tauri build` for both Windows and macOS — full production build
- Test installers on fresh machines (no dev environment)
- Verify PDFium binary is bundled correctly in both installers
- Verify app starts without any developer tools installed
✅ Done when: installer packages work on clean Windows and macOS machines

---

## Quick Reference

| Feature | Days | Issues |
|---------|------|--------|
| PDF ingestion + viewer | 1–10 | — |
| Font embedding check | 11–15 | #21 |
| Page box checks | 16–20 | #27 |
| Image DPI detection | 21–30 | #22 |
| Bleed detection + fixup | 31–35 | #24 |
| PDF/X compliance | 36–50 | #28, #29, #30 |
| Content stream tokenizer | 51–54 | — |
| Color space detection | 55–58 | #23 |
| Overprint + transparency | 59–62 | #25 |
| Hidden content | 63–64 | #26 |
| lcms2 setup | 65–66 | — |
| RGB→CMYK conversion | 67–70 | #34 |
| OutputIntent + PDF/X wizard | 71–72 | — |
| PDF viewer upgrades | 81–85 | #55 |
| Layer management | 86–87 | #35 |
| Page editing | 88–90 | #36 |
| Text search + replacement | 91–96 | #31 |
| Image replacement + editing | 97–100 | #32 |
| Preflight profiles | 101–107 | #39 |
| Action lists | 108–112 | #38 |
| Batch processing | 113–117 | #40 |
| Action list debugger | 118–122 | #41 |
| Hot folders | 123–129 | #42 |
| PDF compression | 131–140 | #49 |
| Barcode detection | 141–145 | #48 |
| Analytics dashboard | 146–149 | #50 |
| Approval sheets | 150–152 | #60 |
| Report export | 153–154 | #59 |
| AI visual checking | 155–158 | #45 |
| Ink coverage | 159 | — |
| Email + FTP integration | 161–166 | #54 |
| MIS webhook | 167–168 | #52 (partial) |
| Keyboard shortcuts | 169–170 | #58 |
| Help system | 171–176 | #57 |
| Polish + QA | 177–195 | — |

