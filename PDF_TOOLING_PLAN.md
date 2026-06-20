# PDF Tooling Implementation Plan

> **Execution model:** This plan is executed by an AI coding agent, not a human on a daily cadence. There is no day/week schedule — work is organized by **phase → workstream → task**, and every workstream is gated by explicit **Done when** acceptance criteria. The agent should complete a workstream, prove the exit criteria, open a PR, and only then advance. Phases are sequential; workstreams within a phase may be parallelized when they share no files.
>
> **Stack:** Tauri v2 (Rust backend + React 19 / TypeScript / Vite frontend), PDFium (`pdfium-render`) + `lopdf` for parsing, `lcms2` for color, SQLite (rusqlite) for local persistence. Windows 11 + macOS 12.0+ (Apple Silicon).
>
> **Status:** Phase 1 workstreams 1.0–1.3 (ingestion/viewer, font check, page boxes, image DPI foundation) are complete — equivalent to the first ~26 work sessions of the prior schedule. Remaining Phase 1 workstreams (bleed fixup, PDF/X compliance, inspector, findings store) and Phases 2–6 are open.

---

## Product Context — Where PDF Tooling Lives

The PDF tooling is one capability inside a **local-first print-shop operations app** (Tauri + TypeScript desktop, SQLite source of truth, hosted cloud as backup-only in v1). Understanding the surrounding product keeps the PDF features integrated rather than siloed.

### Target & positioning
- **Product type:** Multi-tenant SaaS (V2), starting as a **local-first v1** pilot.
- **ICP:** Single-location digital print shops working paper-ish materials up to **13×19**.
- **Intake channels:** Email (paste), walk-in, phone.
- **Proofing:** Default **No** (art approval is optional, not gating).
- **Offline constraint:** One computer is the source of truth.
- **Cloud:** Hosted by us; **backup-only in v1**, sync later.
- **Color model at the job level:** **B/W or Color.**
- **Positioning wedge:** "Fastest, cleanest intake → idiot-proof job ticket → production board" for small digital print shops. Expand into estimating and true multi-tenant SaaS after workflow adoption. Benchmarks: Printavo / shopVOX (lightweight, adoption-first) vs. EFI Pace / PrintSmith / Ordant / Tharstern / Avanti (deep MIS, heavy). We win on intake speed and clarity.
- **Open-source posture:** Build a custom app on OSS primitives (SQLite locally; Postgres/object-storage/queues for hosted backup-sync later; QR + PDF libs; observability later) rather than depending on scarce open-source "print MIS" projects.

### How PDF tooling connects to the job model
The preflight/fixup engine operates on artwork files attached to **jobs**. The existing data model (research-derived, multi-tenant-ready) is the backbone the PDF features read from and write to:

- **`pdf_jobs`** (PDF tooling's own ingestion history) links to **`jobs`** (the shop's work orders) via an optional `client_id` / job association — this is what powers per-client analytics (Phase 5) and approval-sheet auto-fill (Phase 5).
- Outbound PDF actions (email report, FTP upload, MIS webhook in Phase 6) are the same integration surface the broader app uses for job lifecycle events.
- The append-only **`events`** log and **backup_state** discipline (below) apply to PDF-tooling tables too: schema changes ship as ordered migrations; preflight runs and fixups are recorded so cloud backup captures them.

### Backbone schema (research — multi-tenant from day one)
All tables carry `tenant_id TEXT NOT NULL` to keep a clean path to multi-tenant SaaS. PDF-tooling tables (defined per-phase below) follow the same convention.

- **`tenants`** — `tenant_id PK`, `name`, `created_at`.
- **`device`** — `device_id PK`, `tenant_id`, `created_at`.
- **`customers`** — `customer_id PK`, `tenant_id`, `name`, `email?`, `phone?`, `created_at`, `updated_at`, `archived` (default 0).
- **`jobs`** — `job_id PK`, `tenant_id`, `intake_source` (email|walk_in|phone), `customer_name`, `product_type` (flyer|poster|menu|business_card|booklet|other), `quantity`, `size_w_in?`, `size_h_in?`, `size_preset?` (Letter, 11x17, 12x18, 13x19), `stock`, `color_mode` (bw|color), `add_ons_json` (default `[]`), `promised_at` (ISO), `status`, `internal_notes`, `customer_notes`, `created_at`, `updated_at`.
  - **Status enum:** `new → waiting_on_customer → ready_for_production → printing → finishing → ready_for_pickup_or_ship → completed`.
- **`files`** — `file_id PK`, `tenant_id`, `job_id`, `kind` (artwork|proof|other), `original_name`, `mime_type`, `local_path`, `size_bytes`, `sha256`, `created_at`. *(Artwork stored in the app data directory; DB stores metadata + paths + hashes. PDF tooling reads artwork from here.)*
- **`events`** (append-only) — `event_id PK`, `tenant_id`, `device_id`, `sequence` (monotonic per device), `entity_type` (job|customer|file|tenant), `entity_id`, `event_type`, `payload_json` (FULL ROW AFTER CHANGE), `created_at`.
  - **Event types:** Job — `job_created`, `job_updated`, `job_status_changed`, `job_archived?`. Customer — `customer_created`, `customer_updated`. File — `file_attached`.
- **`backup_state`** — `tenant_id PK`, `last_uploaded_sequence` (default 0), `last_snapshot_sequence` (default 0), `last_backup_at?`, `last_snapshot_at?`.
- **Indexes:** `events(tenant_id, device_id, sequence)`; `jobs(tenant_id, status, promised_at)`; `files(tenant_id, job_id)`.

### Offline validation rules (research)
- Required on job create: `customer_name`, `product_type`, `quantity`, `stock`, `promised_at`, `status`.
- Cannot move to `ready_for_production` unless: `quantity > 0`; size is either `size_preset` OR (`size_w_in` AND `size_h_in`); `stock` not empty.
- Proofing not required in v1.

### Ticket generation (research — adjacent to PDF tooling)
The job ticket is a deterministic PDF + OS print dialog (HTML template + print stylesheet), and must include: Job ID + QR deep link (`myapp://job/<job_id>` or internal route); customer name; product type; quantity, size, stock, color, add-ons; promised date/time; internal notes; file list. *(Shares the `printpdf` layout tooling introduced for batch/approval/report export in Phases 4–5.)*

### Backup-only cloud API (research — informs Phase 6 outbound design)
No user portal in v1; one shared **Shop Sync Key** per machine.
- **Headers:** `X-Shop-Key`, `X-Tenant-Id`, `X-Device-Id`.
- **Endpoints:** `POST /v1/events/batch` (idempotent insert by `event_id`); `POST /v1/snapshots` (gzipped JSON, includes `event_sequence_upto` + checksum); `GET /v1/snapshots`; `GET /v1/snapshots/:snapshot_id`.
- **Client scheduling:** upload events every 1–5 min when online + on critical actions; upload snapshot daily + manual "Snapshot now." No cloud→local sync in v1 (restore is manual disaster-recovery).
- **Product roadmap milestones (research):** Offline-first Jobs MVP + board + ticket printing + event log → product templates + add-on checklists + recipes/reorders → Estimating v1 (pricing matrices + add-ons) + quote→job → (V2) true sync, multi-tenant logins/roles, multi-device, deeper estimating. PDF tooling phases run alongside this product spine.

---

## Cross-Cutting Engineering Standards

These apply to **every** phase and workstream — the SaaS/desktop-product quality bar. Treat them as part of each task's definition of done, not optional polish.

### Code health & architecture
- Keep PDF logic in `src-tauri/src/pdf/` as pure, testable functions; keep Tauri `commands` thin (parse args, call core, map errors). Core logic must be unit-testable without a running app.
- Every public command returns `Result<T, AppError>` with a typed, user-facing error enum — never raw `String` debug output (see Error handling).
- Run `cargo fmt`, `cargo clippy -- -D warnings`, `prettier`, and `tsc --noEmit` clean before every PR. No warnings merged.
- Conventions (repo): Tauri commands in `src-tauri/src/commands.rs`, registered in `lib.rs`; DB methods in `db.rs`; Rust structs in `models.rs`; frontend types mirror Rust models in `src/types.ts`; all components use `src/design-system/`; dates stored as `YYYY-MM-DD` strings, compared as strings.

### Testing & golden-file validation
- **Unit tests** for every parser, matrix/CTM op, color transform, and severity rule.
- **Golden-file (snapshot) tests:** maintain a versioned corpus of real PDFs (good, bad, PDF/X-1a, RGB, missing fonts, no bleed, CJK, encrypted, corrupted). Assert findings against committed expected-output JSON. Cross-check against Acrobat Pro / pdfToolbox; document tolerated deltas.
- **Regression gate:** the corpus runs in CI on every PR. A change that alters a golden result must update the snapshot deliberately, with justification in the PR.
- **Fuzzing:** run malformed/truncated/adversarial PDFs through the parser (cargo-fuzz or a seeded corpus) — the app must never panic or hang on hostile input.

### CI/CD, signing & release
- CI builds Windows + macOS on every PR: lint, test, golden corpus, and a production `cargo tauri build` smoke test.
- Releases are **code-signed** (Windows Authenticode) and **notarized** (macOS) so installers run without security warnings.
- Semantic versioning; auto-generated changelog from conventional commits.

### Auto-update
- Ship the Tauri updater wired to signed release artifacts. Updates checked on launch, applied on restart, verified by signature before install. Provide a manual "Check for updates" action.

### Observability
- **Structured logging** (`tracing`) with per-command spans and timings, written to a rotating local log file; expose a "Reveal logs" action for support.
- **Crash reporting:** integrate an opt-in crash reporter (e.g., Sentry) capturing Rust panics and frontend errors with stack traces and app version.
- **Opt-in product telemetry only:** anonymous feature-usage and performance counters, off by default, behind an explicit consent toggle. Never transmit file contents, file names, or client data.

### Security & secrets
- Store all credentials (SMTP, FTP, webhook tokens, AI API keys, Shop Sync Key) in the **OS keychain/credential manager** — never in plaintext SQLite or config files.
- Treat every input PDF as untrusted: enforce recursion-depth and resource limits in the content-stream parser, cap memory, and time-box long operations.
- Principle of least privilege in `tauri.conf.json` (scoped fs access, no unnecessary shell/HTTP allowlist entries).

### Data, migrations & backup
- All schema changes ship as **ordered, idempotent migrations** with a `schema_version` table; migrations run on launch and are forward-only. All tables carry `tenant_id`.
- Writes that modify PDFs are **atomic** and **never overwrite the source** — write to a temp file, fsync, then rename; output uses a suffixed filename.
- Provide DB backup/restore (export the SQLite file) and auto-snapshot before each migration. Mutations append to the `events` log so the backup-only cloud API captures them.

### Privacy & consent (cloud/AI features)
- Any feature that sends data off-device (AI visual check, webhooks, email, FTP, cloud backup) requires **explicit per-feature consent** and a clear statement of what leaves the machine. Customer artwork is sensitive — default cloud features off and surface cost/data implications before first use.

### Licensing & feature flags
- Gate premium/experimental capabilities behind feature flags so workstreams can merge dark and roll out incrementally.
- Reserve a licensing/activation seam early (even if v1.0 ships unlocked) so monetization does not require re-architecture.

### UX invariants
- **Performance budgets** (verified in CI on reference hardware — 6th-gen Core i5, 8GB RAM): open PDF < 2s; 20-page thumbnail strip < 5s; 50-page full preflight < 10s.
- **Accessibility:** every workflow completable by keyboard; status never conveyed by color alone (always icon + label); ARIA labels on icon-only controls.
- **i18n-ready:** no hardcoded user-facing strings in components — route through a string table from the start, even if only English ships.
- **Errors are recoverable & actionable:** inline banners (never `alert()`), each error states what failed and the next action.

---

## Phase 1 — Preflight Foundation

*Issues: #21, #22, #24, #27, #28, #29, #30*

**Goal:** Open, render, inspect, and preflight any PDF, with findings persisted and reportable. This is the product's core value loop.

**Exit criteria:** The app opens, inspects, and preflights arbitrary PDFs; findings match the reference tool on the golden corpus; a combined report renders and persists across restart.

### 1.0 — PDF ingestion & viewer  ✅ *complete*
*Foundation for everything downstream.*

- **Dependencies & PDFium wiring:** add `pdfium-render = { version = "0.9", features = ["sync"] }` and `lopdf = "0.41"` to `src-tauri/Cargo.toml`; download pre-built PDFium binaries (`pdfium.dll` x64, `libpdfium.dylib` arm64) from pdfium-render releases into `src-tauri/resources/`; reference them in `tauri.conf.json` under `bundle.resources`. `cargo check` clean.
- **PDF engine:** `src-tauri/src/pdf/mod.rs` houses all PDF logic. `PdfEngine` wraps native init and loads the bundled binary via `Pdfium::bind_to_library(path)`; `PdfEngine::init() -> Result<Self, AppError>`. Initialize once at startup via `manage()` in `lib.rs`. Smoke test: open a known PDF, assert page count > 0, no panic.
- **`open_pdf(path) -> Result<PdfSummary, AppError>`** — load via PDFium; extract page count and version (`%PDF-x.x` header); read Info dictionary (title/creator/producer/creation date) via lopdf; detect `Encrypt` key in trailer; get file size via `std::fs::metadata`. Register in `lib.rs`.

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

- **`pdf_jobs` persistence:** self-managing table capped at the 20 most recent (delete oldest on insert when count > 20). `db::save_pdf_job(&PdfSummary) -> Result<i64>`, `db::list_pdf_jobs() -> Result<Vec<PdfSummary>>`, `db::delete_pdf_job(id)`; Tauri commands `list_pdf_jobs` / `delete_pdf_job`. Schema mirrors `PdfSummary` plus `opened_at`. *(Add `tenant_id` and optional `client_id` per cross-cutting + analytics needs.)*

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
);
```

- **Rendering commands:** `render_page_thumbnail(path, page_index, width_px=120) -> Result<String>` (render to bitmap, write temp PNG, return path); `render_page(path, page_index, dpi=144) -> Result<String>` (full-size render, default 144 DPI retina / 72 standard).
- **Frontend `PDFView`** (`src/components/PDFView.tsx` + `.css`): add `'pdf'` to the `Section` type in `ManagementView.tsx` and `{ id: 'pdf', label: 'PDF Tools', icon: '📄' }` to `NAV_ITEMS`. File picker via `@tauri-apps/plugin-dialog` with PDF filter (`{ name: 'PDF', extensions: ['pdf'] }`); metadata card (name, page count, version, creator, encrypted badge); recent-files sidebar from `list_pdf_jobs`. `ThumbnailStrip` (first 20, rest lazy-loaded on scroll, click-to-navigate). `PageViewer` with zoom presets (fit-to-width, fit-to-page, 50/75/100/150/200%), prev/next + jump-to-page, debounced re-render (300 ms after last zoom change).
- **Recent files UX:** "Remove from history" and "Open Again" (re-runs `open_pdf` with stored path) per row; "File not found" state when the path no longer exists; sort by `opened_at` descending.
- **Edge cases / errors:** encrypted → `"PDF is encrypted — password required"`; corrupted/truncated → descriptive parse error; zero-page → guard empty page tree; missing Info fields default to `""`; zero-byte `.pdf`. All surfaced as **inline banners**, never `alert()`, never a crash.

**Done when:** any valid PDF opens to a populated metadata card with navigable thumbnails and zoomable pages; encrypted/corrupted/zero-page files show actionable inline errors without crashing; recent files persist and self-manage.

### 1.1 — Font embedding check (#21)  ✅ *complete*

```rust
pub struct FontFinding {
    pub font_name: String,
    pub font_type: String,   // "Type1" | "TrueType" | "CIDFont" | "OpenType"
    pub is_embedded: bool,
    pub is_subsetted: bool,
    pub pages: Vec<usize>,   // 1-indexed
    pub severity: String,    // "error" | "warning" | "ok"
    pub message: String,
    pub fix_hint: String,
}
```

- **PDFium enumeration:** per page, walk `PdfPageFonts` — `is_embedded()`, `family_name()`, `font_type()`. Detect subset prefix via regex `^[A-Z]{6}\+`. Deduplicate by name, accumulate page list.
- **lopdf fallback & merge:** walk `Page.Resources.Font` → `FontDescriptor` for `FontFile` / `FontFile2` / `FontFile3`; handle Type0 composite fonts via `DescendantFonts`. Cross-reference: if lopdf finds a font PDFium missed, add it. Ensures CJK CIDFonts are captured.
- **`check_fonts(path) -> Result<Vec<FontFinding>>`** severity: not embedded → error (`"Font '{name}' is not embedded. The receiving printer may substitute a different font."`); embedded but not subsetted → warning (`"Font '{name}' is fully embedded (not subsetted). Consider subsetting to reduce file size."`); embedded + subsetted → ok. Every finding carries an actionable `fix_hint` (e.g., InDesign/Illustrator/Acrobat embedding steps).
- **`FontCheck` UI** (`src/components/preflight/FontCheck.tsx` + `.css`): summary counts (N found / N errors / N warnings); table name | type | embedded | subsetted | pages | status badge (red/yellow/green); All vs. Issues-only filter; wired into `PDFView` as a collapsible section.

**Done when:** mixed embedded/unembedded and CJK CIDFont test files report complete, correctly-classified font lists with fix hints.

### 1.2 — Page box checks (#27)  ✅ *complete*

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

- **Extraction (lopdf):** read each page dictionary with **inheritance** (walk up parent nodes; dereference indirect refs via `Document::dereference`); convert points→mm (`× 0.3528`); handle rotated pages (swap W/H when `Rotate` = 90/270).
- **Validation:** MediaBox missing → error; TrimBox missing → warning (required for PDF/X); BleedBox present but smaller than TrimBox on any side → error; BleedBox absent when TrimBox present → info (error only under PDF/X); TrimBox larger than MediaBox → error; mixed page sizes across the document → warning (flag inconsistent pages). Bundle standard bleeds: 3 mm (ISO), 0.125″ (US), configurable.
- **`check_page_boxes(path) -> Result<Vec<PageBoxFinding>>`** with a document-level summary (all pass / N pages with issues). Register in `lib.rs`.
- **`PageBoxCheck` UI:** per-page rows (page #, boxes present as checkmarks, issues); expand a row → all box dimensions in mm; nested-rectangle SVG (Media ⊃ Bleed ⊃ Trim); summary line ("All pages have correct bleed" / "3 pages missing TrimBox").

**Done when:** known-bad box files are flagged correctly and box dimensions match Acrobat to the mm; indirect-ref boxes, inherited resources across page subtrees, and rotated pages all handled.

### 1.3 — Image DPI / resolution detection (#22)  ✅ *complete (foundation)*

Builds the **content-stream foundation reused across the product**.

- **`GraphicsState`** in `src-tauri/src/pdf/content_stream.rs`: `ctm: [f64; 6]`, `stack: Vec<[f64;6]>`; `identity()`, `push()`/`pop()` for `q`/`Q`, `apply_cm(a,b,c,d,e,f)` with matrix multiply. Unit-tested for stacking and multiplication.
- **Tokenizer** `tokenize(bytes) -> Vec<Token>`: `Token` enum `Number(f64)`, `Name(String)`, `StringLit(Vec<u8>)`, `Operator(String)`, `ArrayStart/End`, `DictStart/End`. Handle comments (`%`→EOL), hex strings (`<…>`), literal strings (`(…)`), names (`/Name`); operators are letters only (`cm`, `Do`, `Tf`, `q`, `Q`, `rg`, `k`).
- **Executor** `execute_stream(tokens, &mut GraphicsState)`: handle `q`/`Q`/`cm` for CTM; collect the operand stack (numbers + names) preceding each operator.
- **`Do /XObject`:** record name + CTM at invocation; look up in page Resources/XObject. For image XObjects, retrieve `Width`/`Height`, compute rendered size from CTM (`rendered_w = sqrt(a² + c²) × (Width_pts / 72)`) and `effective_dpi = pixel_width / (rendered_width_pts / 72)`.

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
    pub severity: String,   // "error" | "warning" | "ok"
    pub message: String,
}
```

- **`check_image_resolution(path, min_dpi) -> Result<Vec<ImageResolutionFinding>>`:** iterate pages, run the parser, collect `Do` image usages with CTM, look up Width/Height/ColorSpace in lopdf, apply thresholds (default <150 error, 150–299 warning, 300+ ok — all configurable).
- **Recursion & inline images:** recurse into Form XObjects (apply their `Matrix` to CTM before recursing; cap depth at 10 for malformed-PDF protection); handle inline images (`BI … EI`, parse inline dict for Width/Height/ColorSpace, use current CTM).
- **Multiple usages:** report each `Do` usage independently (same image at 72 DPI in one place and 300 DPI in another → two rows); deduplicate identical usages at the same CTM (generators sometimes emit duplicate `Do`).
- **`ImageResolutionCheck` UI:** configurable min-DPI slider (72/150/300/600); summary (N checked / N below threshold); table page | image ID | pixel dims | rendered mm | DPI | color space | status, sorted worst-DPI-first; re-runs on slider change.
- **Performance:** parallelize per-page parsing with `rayon` (`par_iter` over pages; add `rayon = "1"`). 100-page / 200-image file under 3 s (perf budget).

**Done when:** low-DPI images (including images reused at different sizes and wrapped in Form XObjects, plus inline images) are reported with correct per-usage DPI within the perf budget.

### 1.4 — Bleed detection & add-bleed fixup (#24)

```rust
pub struct BleedFinding {
    pub page: usize,
    pub has_bleed_box: bool,
    pub bleed_top_mm: f64,
    pub bleed_right_mm: f64,
    pub bleed_bottom_mm: f64,
    pub bleed_left_mm: f64,
    pub min_required_mm: f64,
    pub severity: String,
}
```

- **`check_bleed(path, min_bleed_mm) -> Result<Vec<BleedFinding>>`:** measure all four sides independently `((BleedBox.coord − TrimBox.coord) × 0.3528)`; flag each insufficient side with a precise message (`"Right bleed is 2.1mm — minimum is 3mm"`); treat **ArtBox as the trim reference when TrimBox is absent**; respect page rotation for side mapping.
- **`add_bleed(path, amount_mm, output_path) -> Result<()>`:** expand BleedBox by `amount_mm` on all four sides (derive from TrimBox if absent), grow MediaBox to contain the new BleedBox. **Atomic write to a new suffixed file — never overwrite input.**
- **`BleedCheck` UI:** per-page side table (page | top | right | bottom | left | status) with side-specific highlighting; red/green margin SVG of the page box + bleed; "Add Bleed" fixup panel (amount mm, output filename, run). On load runs `check_bleed`; shows the add-bleed action when any side fails.
- **Edge cases:** no bleed at all; correct bleed; partial bleed; corrupted (BleedBox smaller than TrimBox); rotation affecting which side is "top" vs "right".

**Done when:** check → add-bleed → re-check round-trips to a pass, and output opens correctly in Acrobat with correct box sizes.

### 1.5 — PDF/X compliance, inspector & findings store (#28, #29, #30)

- **OutputIntent extraction** `get_output_intents(doc) -> Vec<OutputIntent>` (`s_key` = GTS_PDFX/GTS_PDFA, `output_condition`, `output_condition_id`, `registry_name`, `has_embedded_icc`, `icc_num_channels`): walk catalog → `OutputIntents` array → each dict; check `DestOutputProfile` stream presence.
- **Metadata checks** `check_metadata(doc) -> Vec<PdfXFinding>`: `GTS_PDFXVersion` in Info (parse + validate); `Trapped` must be `/True`, `/False`, or `/Unknown`; header version floors (X-1a ≥ 1.3, X-4 ≥ 1.6).
- **Security / forbidden content:** `Encrypt` in trailer → error; scan page annotations for Sound/Movie/Screen/Widget (unless non-printing); JavaScript via `Names` → `JavaScript` name tree and page `AA` additional actions; OPI keys in XObject dictionaries.
- **`check_pdfx(path, profile) -> Result<Vec<PdfXFinding>>`** assembler dispatching `x1a | x3 | x4` — runs fonts (all embedded), page boxes (TrimBox/ArtBox on all pages), output intent (GTS_PDFX present), metadata, and security; returns a combined, deduplicated finding list. **Profile differences:** X-1a — color check stub ("⚠ Color space validation runs in Phase 2"), transparency forbidden; X-3 — "ICC-managed RGB allowed" (still stubbed in Phase 1); X-4 — as X-3 but live transparency allowed (transparency check removed). The color stub is removed and wired live in Phase 2.
- **Deep inspector (#29)** `get_pdf_catalog(path) -> Result<serde_json::Value>` returns the catalog as JSON. `PdfInspector` (`src/components/preflight/PdfInspector.tsx`): object browser (catalog as expandable tree, clickable refs → raw dict/stream info); tabs **Document Info** (Info dict), **Page Tree** (count + size summary), **Resources** (fonts/XObjects/colorspaces). Page-level detail: select a page → its dictionary, per-page Resources/Font, Resources/XObject, Resources/ColorSpace, MediaBox, TrimBox, stream length + compression filter; "View Raw Stream" shows decoded bytes as text for any XObject/content stream.
- **Findings persistence (#30):**

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
);
-- plus preflight_run_summary(job_id, profile, total_errors, total_warnings, total_ok, ran_at)
```

  `db::save_findings(job_id, findings)`; `list_findings_for_job(job_id)`; `list_preflight_runs(job_id)`. History reloads stored runs **without re-running** the check.
- **Combined report** `PreflightReport` (`src/components/preflight/PreflightReport.tsx`): top pass/fail banner (PASS all-green / FAIL with N errors, N warnings, color-coded); collapsible sections (Fonts / Page Boxes / Bleed / PDF/X Metadata / Security), each with a section-level badge; per-finding row (severity icon + check name + page + message + `fix_hint`); "Run Full Check" with profile selector (X-1a / X-3 / X-4 / Custom); **auto-save** summary + findings on run with "Saved at [timestamp]" and a new `preflight_run_summary` row per run; `PreflightHistory` list (past runs with pass/fail badge + timestamp, click to reload stored findings).
- **Sidebar badge & a11y:** error/warning count badge next to "PDF Tools" in `ManagementView` (red circle for errors, yellow for warnings) pulled from the most recent run for the open file; full keyboard navigation (←/→ page, +/− zoom, Cmd/Ctrl+O open, Cmd/Ctrl+R run preflight, Esc close expanded finding) with shortcut hints in tooltips.
- **Finding-message polish:** every error states what it means, what happens at print, and how to fix it in InDesign/Illustrator/Acrobat; `fix_hint` field on all Finding structs.

**Done when:** a known PDF/X-1a file reports mostly-pass (with the color stub noted); the inspector shows catalog/page detail for any file; running twice yields two history entries; the combined report renders and persists across restart; the sidebar badge reflects the latest run.

### 1.6 — Phase 1 integration & golden baseline

- Gather 10 real-world PDFs (good, bad, PDF/X-1a, RGB, missing fonts, no bleed, large 100+ page, small no-metadata). Run full preflight on each; verify findings match Acrobat Pro; fix discrepancies. Commit these as the **golden baseline** the regression gate compares against in later phases.
- `cargo check`, `cargo clippy -- -D warnings`, `npx tsc --noEmit` clean. PR: "Phase 1 complete — PDF preflight foundation."

**Done when:** Phase 1 PR merged; the app opens, inspects, and preflights any PDF; results captured as the baseline corpus.

---

## Phase 2 — Color Space Detection & Conversion

*Issues: #23, #25, #26, #34*

**Goal:** Detect every color space, overprint, transparency, and hidden-content risk, then convert RGB→CMYK and assemble valid PDF/X, defaulting to PDF/X-4.

**Exit criteria:** Color/overprint/transparency/hidden-content checks integrate into the PDF/X assembler; the "Make PDF/X-4" wizard turns a typical RGB export into a valid PDF/X-4 with live transparency preserved (minus font subsetting, deferred to Phase 5); results match the reference tool on 8/10 corpus files.

### 2.1 — Color space detection (#23)

- **Full color operator set** in `content_stream.rs`: stroke `CS`, `SC`/`SCN`, `G`, `RG`, `K`; fill `cs`, `sc`/`scn`, `g`, `rg`, `k`; named spaces from `Resources/ColorSpace`; `gs /Name` → `Resources/ExtGState`. Unit-test each operator.
- **`resolve_color_space(name, resources) -> ColorSpaceKind`** enum: `DeviceGray`, `DeviceRGB`, `DeviceCMYK`, `CalGray`, `CalRGB`, `Lab`, `ICCBased(channels)`, `Separation(alt)`, `DeviceN(alt)`, `Indexed(base)`, `Pattern`, `Unknown`. Resolve array form (`[/ICCBased stream_ref]`), name form, indirect refs; recursively resolve base spaces for Indexed/Separation; recurse into Form XObjects with their own Resources.
- **`ColorUsage`** accumulator (`color_space_kind`, `is_stroke`, `is_fill`, `page`, `object_context`): `execute_stream` populates `Vec<ColorUsage>`, tracking current stroke/fill space and updating on each color operator.

```rust
pub struct ColorSpaceFinding {
    pub color_space: String,
    pub kind: String,             // "stroke" | "fill"
    pub pages: Vec<usize>,
    pub is_pdf_x_violation: bool,
    pub severity: String,
    pub message: String,
}
```

- **Per-profile classification:** X-1a — any DeviceRGB/CalRGB/Lab/ICCBased → error; X-3/X-4 — RGB/Lab without embedded ICC → error (ICC-managed allowed); non-PDF/X CMYK job containing RGB → warning ("Job appears CMYK-intended but contains RGB objects").
- **ICC header parsing** for `ICCBased`: read profile header (first 128 bytes) — color-space signature at offset 16 (`RGB ` 0x52474220, `CMYK` 0x434D594B, `GRAY` 0x47524159), profile class, channel count (PDF `N` key), `desc` description. Identify sRGB vs FOGRA39 from binary headers.
- **`check_color_spaces(path, target_profile) -> Result<Vec<ColorSpaceFinding>>`** (`pdfx_1a | pdfx_3 | pdfx_4 | cmyk_only | any`): list all spaces found, highlight per-profile violations. **Remove the Phase 1 color stub** and wire the real result into `check_pdfx`.
- **`ColorSpaceCheck` UI:** found-spaces chips (DeviceCMYK ✓, DeviceRGB ✗, ICCBased ✓); table space | type | stroke/fill | pages | compliant-with-profile (violations in red); ICC profile detail (name, channels, colorspace type) for ICCBased spaces.

**Done when:** mixed RGB+CMYK files break down correctly by space/usage; ICC profiles (sRGB vs FOGRA39) are identified from binary headers; PDF/X color violations surface in the assembled report.

### 2.2 — Overprint, transparency & hidden content (#25, #26)

- **Overprint:** track `ExtGState` from `gs` — `OP` (stroke), `op` (fill), `OPM` (0 = knockout, 1 = non-zero). `OverprintFinding` (page, object_context, overprint_stroke, overprint_fill, mode, severity, message): 0% ink + overprint → error ("White knockout missing — 0% ink with overprint will show through"); overprint on RGB → error (meaningless, ignored by RIP); K-only black overprint → info. `check_overprint(path)` + a collapsible report section; an "Overprint Preview" toggle placeholder in the viewer (live simulation in Phase 3).
- **Transparency:** detect `Group` dicts in page dictionaries; scan `ExtGState` for `ca` (fill) / `CA` (stroke) < 1.0 or `BM` other than `/Normal`/`/Compatible`. `TransparencyFinding` (page, type opacity/blend_mode, value, is_pdfx1a_violation, severity): X-1a → error, X-4 → info. `check_transparency(path)` wired into the report and the X-1a compliance failure path.
- **Hidden content:** off-page objects (track path coords from `m`/`l`/`c` and text from `Td`/`TD`/`Tm` vs MediaBox); default-off Optional Content Groups (layers); white-on-white (needs color + graphics-state tracking). `check_hidden_content(path) -> Vec<HiddenContentFinding>` (page, type off-page / default-off-layer / white-on-white, description). Report section.
- Integrate all three into `PreflightReport` and the PDF/X assembler; total error/warning counts update dynamically.

**Done when:** a drop-shadow file flags live transparency, a 0%-ink-overprint file flags white-knockout, and off-page crop marks are reported as hidden content.

### 2.3 — Color management & RGB→CMYK conversion (#34)

- **lcms2 FFI** (`lcms2-sys = "0.7"`; verify Windows build, may need vcpkg/bundled C source). `pdf::color::transforms::LcmsEngine`: `open_profile_from_bytes(&[u8]) -> Profile`, `create_transform(src, dst, intent) -> Transform`, `transform_pixels(&Transform, src, channels_in) -> Vec<u8>`. Smoke test: sRGB→FOGRA39 RGB triple → expected CMYK.
- **Bundle ICC profiles** as Tauri resources: `sRGB_v4_ICC_preference.icc`; `ISOcoated_v2_eci.icc` (FOGRA39 — most common offset); `USWebCoatedSWOP.icc`; `GRACoL2006_Coated1v2.icc`. `list_icc_profiles() -> Vec<IccProfileInfo>` (name, description, color_space, channels, file_name).
- **`convert_rgb_to_cmyk(path, src_profile, dst_profile, rendering_intent, scope, output_path) -> Result<()>`:** convert image XObjects (decode stream → lcms2 → re-encode JPEG q90; update ColorSpace→`/DeviceCMYK`, Length) and vector colors inline (`rg R G B` → `k C M Y K` via content-stream round-trip: tokenize → modify → re-encode, update stream dict/filter). `scope` = images / vector / both.
- **Edge cases:** indexed (resolve base first, then convert); Lab→CMYK; spot colors (Separation/DeviceN) left intact and flagged ("Spot color: {name} — not converted"); page-level vs global Resources.
- **`add_output_intent(path, icc_profile, condition_id, output_path) -> Result<()>`:** create the OutputIntents array if absent; embed the ICC profile as a stream; set `/S /GTS_PDFX`, `/OutputConditionIdentifier`, `/DestOutputProfile`; set `/GTS_PDFXVersion` in Info if missing.
- **`ColorConversionPanel` UI:** source profile (auto-detected or override); destination (FOGRA39/SWOP/GRACoL or browse for ICC); rendering intent (Perceptual default | Relative Colorimetric | Absolute Colorimetric | Saturation) with plain-language help — Perceptual "best for photos, compresses gamut, maintains visual relationships"; Relative Colorimetric "best for logos/spot approximations, shifts white point, clips out-of-gamut"; Absolute Colorimetric "preserves absolute values, only for simulating one press on another"; Saturation "best for business graphics, maximizes saturation, not color-accurate"; scope (images/vector/both); suffixed output (default `[name]_CMYK.pdf`); progress.
- **Performance:** parallelize image conversion with `rayon`; emit `conversion_progress` events (`{ current, total }`); keep lopdf writes single-threaded (collect converted streams, then write). 50-page / 30-image file converts within budget with a live progress bar.

**Done when:** an RGB file (images + vector + spot) converts to CMYK with spots preserved, opens correctly in Acrobat, and large files convert within budget with a live progress bar.

### 2.4 — "Make PDF/X-4" wizard & spot/ink inventory

- **Flagship fixup wizard:** (1) run full preflight → show issues; (2) toggle each auto-fixable issue (add bleed; embed OutputIntent; optional RGB→CMYK; font subsetting skipped — Phase 5); (3) review summary of changes; (4) apply in sequence to a new file; (5) auto re-run preflight on the output. **PDF/X-4 is the recommended default target.**
- **Why X-4 by default:** X-4 permits live transparency and ICC-based color, so the wizard embeds an OutputIntent rather than flattening transparency or force-converting every RGB object — avoiding the lossy flattening PDF/X-1a requires. Offer **PDF/X-1a as a stricter legacy fallback** for shops/RIPs that still require it (it flattens transparency and converts to CMYK).
- **General Print (non-PDF/X) profile:** RGB and TAC>300% as warnings; no OutputIntent/Trapped requirements; spot colors without defined alternates → warning. The right default for a small shop and a typical RGB flyer.
- **Spot color inventory:** walk all Separation/DeviceN spaces; extract names (e.g., "PANTONE 485 C", "Die Cut", "Varnish"). `SpotColorFinding` (name, pages, has_alternate_colorspace, alternate_colorspace_type); flag process names (Cut/Die/Crease/Varnish) for review in a distinct color. `SpotColorInventory` report section.
- **`check_ink_coverage` stub:** `InkCoverageFinding` (page, max_tac, average_tac, exceeds_threshold); stub returns `Err("Ink coverage requires rendering — available in Phase 5")`; placeholder report section with a "Coming in Phase 5" banner so the API/UI shape stays stable.

**Done when:** a typical InDesign RGB export passes through the wizard to valid PDF/X-4 with live transparency preserved (minus font subsetting); the X-1a fallback still produces a valid flattened file; PANTONE spots are inventoried by name. **Regression:** Phase 1+2 preflight matches the reference tool on 8/10 corpus files with documented discrepancies.

---

## Phase 3 — PDF Viewing & Editing Foundations

*Issues: #31, #32, #35, #36, #55, #56*

**Goal:** Production-grade viewing (separations, overprint preview, measurement) and safe editing (pages, text, images) on a clean content-stream round-trip.

**Exit criteria:** Full editing workflow works end-to-end; content streams round-trip without visual change on a complex page.

### 3.1 — Viewer upgrades & inspection tools (#55, #56)

- **Full-screen viewer:** fit-to-width default; trackpad pinch-zoom (wheel + Ctrl); smooth CSS page transitions; status bar (page N of N, zoom %, file name); toggle between embedded and full-window mode.
- **Overprint preview toggle:** render via PDFium overprint-simulation flag; disclaimer "⚠ approximate — use a RIP for production proof." Produces visibly different rendering for overprinting black.
- **Separation / plate view:** "View Plate" dropdown — CMYK (C/M/Y/K) plus detected spots; isolate a single ink channel (PDFium channel render or grayscale-per-channel canvas/CSS filter). "C plate" shows only cyan content in grayscale.
- **Color picker (eyedropper):** click any rendered point → RGB hex, CMYK-approximate, LAB; caveat that it reads **rendered pixel** values, not document color values. Reads ~(0,0,0,100) on CMYK black.
- **Measurement tool:** two-point distance in mm/in/pt via page CTM; show distance, angle, horizontal/vertical components; unit toggle. Measuring an A4 width returns 210 mm.

**Done when:** plate view isolates a single ink, the picker reads ~(0,0,0,100) on CMYK black, and measuring an A4 width returns 210 mm.

### 3.2 — Layers & page operations (#35, #36)

- **Layers:** `list_layers(path) -> Vec<LayerInfo>` (name, id, is_visible_default, is_locked, intent View/Design/Export). `set_layer_visibility(path, layer_id, visible, output_path)` toggles OCG default state in `/OCProperties/D/ON|OFF`; re-render after. `LayerPanel` with eye-icon toggles; PDFs with named layers (Illustrator export) show layers by name.
- **Page ops (all write suffixed output, never overwrite):** `extract_pages(path, indices, output)`; `delete_pages(path, indices, output)`; `rotate_page(path, index, degrees{90,180,270}, output)`; `reorder_pages(path, new_order, output)`; `insert_blank_page(path, after_index, width_mm, height_mm, output)` — lopdf edits to the Pages tree Kids array and page Rotate key.
- **`PageOperationsPanel`:** multi-select thumbnail grid (Shift/Cmd-click); toolbar Extract / Delete / Rotate / Insert Blank; drag-to-reorder via `@dnd-kit/core`. All operations write `_edited`-suffixed files.

**Done when:** layer toggles change the render, and multi-select delete / drag-reorder produce correctly ordered output files.

### 3.3 — Content-stream round-trip (foundation)

- `decode_stream(doc, page_index) -> Vec<u8>`: handle concatenated content streams; Flate / LZW / DCT filters.
- `encode_stream(tokens) -> Vec<u8>`: serialize tokens back to PDF operator syntax; update the stream dict (remove or re-apply filter).
- **Round-trip golden test:** decode → re-encode a complex page (text, images, paths, transparency) → visually identical in PDFium. **This gates all text/vector editing.**

**Done when:** the round-trip golden test passes for a complex page.

### 3.4 — Text search & replacement (#31)

- `get_text_objects(path, page_index) -> Vec<TextObject>` (text, x, y, width, height bbox in page coords, font_name, font_size) via PDFium text API.
- `search_text(path, query) -> Vec<TextMatch>` (page_index, text, start_char, end_char, bounding_boxes); viewer search bar with highlight-all + next/prev across pages.
- `replace_text(path, page_index, find, replace, output_path) -> Result<ReplaceResult>` editing `Tj`/`TJ` operands; `ReplaceResult { replacements_made, warnings }`. Scope: replacement fitting the same approximate width (typo/date/number fixes); overflow warning when the replacement is longer.
- **Encodings & recursion:** PDFDoc / WinAnsi / MacRoman / UTF-16BE byte handling; recurse into Form XObjects; handle text split across adjacent `Tj` runs; warn on Type3 fonts (glyph-as-content, can't be string-substituted).
- **`TextEditPanel`:** find/replace with page scope (current/all); Find-All list with page + context; Replace / Replace-All; overflow banner; before/after preview (re-render affected page).

**Done when:** typo correction works in simple, CJK (UTF-16BE), and Form-XObject text with visual confirmation.

### 3.5 — Image replacement & optimization (#32)

- `replace_image(path, page_index, xobject_name, new_image_path, output_path) -> Result<()>` via the `image` crate (load PNG/JPEG/TIFF; re-encode DCT/Flate; update Width/Height/ColorSpace/BitsPerComponent/Filter; preserve mask/decode array and other attributes).
- **Click-to-select:** hit-test the rendered bbox to find which XObject contains the click; info readout (name, pixel dims, DPI, color space, compression); "Replace Image" → file picker.
- `optimize_image(path, xobject_name, settings, output_path)`: JPEG quality re-compress; downsample to target DPI (`image::resize`); convert to grayscale.
- **`ImageEditPanel`:** per-image controls (current DPI/size/compression) + "Apply to all low-DPI images" batch action.

**Done when:** an image can be selected, inspected, replaced, and quality-reduced from the UI, producing a valid PDF; full editing workflow works end-to-end.

---

## Phase 4 — Automation Engine

*Issues: #38, #39, #40, #41, #42*

**Goal:** Turn manual checks/fixups into reusable profiles, recorded action lists, batch jobs, hot folders, and a step debugger.

**Exit criteria:** Profiles, action lists, batch, hot folders, and the debugger all work together; concurrent batch + hot-folder runs are safe.

### 4.1 — Preflight profiles & registries (#39)

- **Schema** (with migrations for existing installs):

```sql
CREATE TABLE IF NOT EXISTS preflight_profiles (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL,
  is_builtin INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS profile_checks (
  id INTEGER PRIMARY KEY,
  profile_id INTEGER NOT NULL REFERENCES preflight_profiles(id) ON DELETE CASCADE,
  check_id TEXT NOT NULL, severity TEXT NOT NULL DEFAULT 'error',
  params TEXT NOT NULL DEFAULT '{}', sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS profile_fixups (
  id INTEGER PRIMARY KEY,
  profile_id INTEGER NOT NULL REFERENCES preflight_profiles(id) ON DELETE CASCADE,
  fixup_id TEXT NOT NULL, params TEXT NOT NULL DEFAULT '{}',
  condition_check_id TEXT, sort_order INTEGER NOT NULL DEFAULT 0
);
```

- **Seed 4 built-ins** (`is_builtin=1`, clone-only): **PDF/X-4** *(default)*; **PDF/X-1a** (legacy fallback, flattens transparency); **General Print (CMYK)** (RGB → warning, no OutputIntent required); **Wide Format (RGB OK)** (image DPI min 100, RGB allowed, no bleed required). The app's **default preflight profile is PDF/X-4**.
- **Profile CRUD** (`list_profiles`, `get_profile`, `create_profile`, `update_profile`, `delete_profile`) + `clone_profile(id, new_name)`.
- **Static registries:** `CHECK_REGISTRY: &[CheckDefinition]` (`id`, `label`, `description`, `default_severity`, `params_schema` JSON) covering all Phase 1+2 checks; `FIXUP_REGISTRY` (`add_bleed`, `convert_rgb_to_cmyk`, `add_output_intent`, `replace_text`, …) with optional `condition_check_id` (apply only to objects failing a specific check). `list_check_definitions()`.
- **`ProfileEditor` UI:** profile list (built-ins locked); detail panel with drag-orderable checks/fixups (separate tabs), severity selectors, params rendered from JSON schema (`number` → input w/ min/max; `string`+`enum` → select; `boolean` → checkbox); Clone/Delete on user profiles; searchable "Add Check" picker.
- **`run_profile(path, profile_id, output_path?) -> Result<ProfileRunResult>`:** run fixups in `sort_order` (pipe output→input), then checks on the post-fixup file, persist findings under a new run record, return applied fixups + findings + output path.

**Done when:** a custom profile with a subset of checks and custom thresholds (e.g., min_dpi=150) runs, applying fixups before checks and producing a fixed file that passes.

### 4.2 — Action lists: record, replay, edit (#38)

- **Schema:** `action_lists(id, name, description, created_at)`; `action_list_steps(id, action_list_id FK CASCADE, action_type "check"|"fixup", action_id, params '{}', sort_order, created_at)`.
- **Record mode:** "Record" captures each invoked command (replace_text, add_bleed, convert_rgb_to_cmyk, …) as an ordered step; recording indicator (red dot); "Stop" saves a named list.
- **`run_action_list(action_list_id, path, output_path) -> Result<ActionListResult>`:** execute steps in order (pipe output→input); `ActionListResult { steps: Vec<StepResult> }` where `StepResult { success, message, output_size_bytes }`.
- **`ActionListPanel`:** saved lists with step count; step detail (type, name, params); "Run on Current File"; drag-reorder / edit-params / delete; "Record New".

**Done when:** a recorded add-bleed → convert-colors list replays to the same result as the manual steps.

### 4.3 — Batch processing (#40)

- **Schema:** `batch_jobs(id, name, input_folder, output_folder, profile_id?, action_list_id?, status 'idle', created_at, last_run_at?)`; `batch_results(id, batch_job_id FK CASCADE, file_name, status "pass"|"fail"|"error", error_count, warning_count, output_file?, processed_at)`.
- **`run_batch(batch_job_id)`** (async): iterate `.pdf` files in `input_folder`, apply profile/action list, write per-file results, emit `batch_progress` (`{ current, total, file_name, status }`); handle file-access errors, malformed files, disk-full.
- **Pass/fail routing:** optional fail folder; passing files → output (pass) folder, failing → fail folder (copy or move, configurable) — the basic hot-folder routing model.
- **Batch summary report PDF** via `printpdf` (job name, date/time, totals, per-file status table) saved as `batch_report_[timestamp].pdf`.
- **`BatchPanel`:** create job (input/output folders, profile or action list); "Run Batch"; live progress bar + current file; results table (file | status | errors | warnings | output link); "Export Summary CSV".

**Done when:** a folder of mixed PDFs processes with live progress, routes pass/fail correctly, and emits a summary report.

### 4.4 — Action list debugger (#41)

- **`ActionListDebugger`:** load an action list + target PDF; "Step Forward" runs one step; current-step highlight; per-step result (success/failure, time, output); re-render after each step.
- **Before/after split render** per step; for checks show the finding list on the current document state; for fixups show what changed (finding list + visual diff); click a finding → jump to page + highlight affected object; "Run from Here" restarts from a step with the current state.
- **Persist debug sessions** (action list + PDF + step index + per-step results) to DB; "Reopen" resumes at the last step after restart.
- **Export debug report PDF:** steps, per-step success/time/finding-count, before/after screenshots.

**Done when:** stepping a multi-step list shows before/after renders and findings, and a session resumes after restart.

### 4.5 — Hot folders (#42)

- **Watcher:** `notify = { version = "7", features = ["serde"] }` + `notify-debouncer-full`. `pdf::watcher::FolderWatcher::start(path, tx)`; debounce write→close→written into one event; filter `.pdf`, ignore temp files (`.tmp`, `~$`, `.crdownload`).
- **Schema:** `hot_folders(id, name, watch_path, pass_path, fail_path, profile_id?, action_list_id?, is_active 1, created_at)` + CRUD; `hot_folder_log`.
- **`start_hot_folder_service()`** at startup (in `lib.rs` `setup`): process each new PDF (run configured profile, route pass/fail), log to `hot_folder_log`, emit `hot_folder_event` for live UI.
- **Resilience (best practice):** retry while a file is still being written (size changing, up to 3×); route hard errors (corrupt, permission-denied) to an `_error` subfolder with a desktop notification (Tauri notification plugin); bounded concurrent processing (default 2) with a visible queue depth — don't process 20 files at once on old hardware.
- **`HotFolderPanel`:** folder list (name | watch path | active toggle | last activity); add/edit/delete; live scrolling activity log; "Open Watch / Pass / Fail Folder" quick links.

**Done when:** dropping files auto-processes and routes them; 10 dropped at once process 2-at-a-time with a queue display; corrupt files land in `_error` with a notification.

### 4.6 — Phase 4 integration

- Full workflow: create a profile with all Phase 1+2 checks → record add_bleed → convert_rgb_to_cmyk → run via batch on 5 PDFs → verify outputs → enable a hot folder, drop a PDF, verify auto-processing → debug the action list. Test concurrent batch + hot folder safety. `cargo check` + `tsc` clean. PR: "Phase 4 complete — automation engine."

**Done when:** all automation features work together without errors; clean merge.

---

## Phase 5 — Advanced Features

*Issues: #45, #48, #49, #50, #59, #60*

**Goal:** Compression (incl. font subsetting), barcode validation, analytics, approval sheets, report export, AI visual checking, and ink coverage.

**Exit criteria:** Compression, barcode, AI visual, analytics, and report export all run from the UI and integrate into the report/registry.

### 5.1 — PDF compression & font subsetting (#49)

- **Settings & analysis:** `image = "0.25"`. `CompressionSettings` (target_image_dpi, jpeg_quality 0–100, compress_streams Flate, subset_fonts, remove_metadata, remove_unused_objects). `analyze_compression_potential(path) -> CompressionAnalysis` — count images, measure image-stream bytes, identify images above target DPI, estimate compressed size.
- **Image downsampling:** `downsample_image(img_data, color_space, w, h, current_dpi, target_dpi) -> (Vec<u8>, u32, u32)` — decode to `image::DynamicImage`, resize maintaining aspect ratio, re-encode JPEG at configured quality. Apply to all images above threshold.
- **Stream re-compression:** re-compress uncompressed/LZW streams with Flate (`miniz_oxide = "0.8"`); remove unreferenced objects from the xref before saving.
- **Font subsetting (the hard part):** `subset_font(font_stream, used_glyph_ids) -> Vec<u8>` using `ttf-parser` — keep only used glyph IDs in `glyf`/`loca`/`hmtx`; rebuild `head`/`hhea`/`maxp` with updated counts; rebuild `cmap` to reference only used glyphs; update `name` with `ABCDEF+` PostScript prefix; preserve hinting (`fpgm`/`prep`). Handle CIDFont Type2 (also update the ToUnicode CMap). Flag OpenType CFF as unsupported (warn, skip). Validation pass first reports waste (e.g., "Font 'Arial' has 1,245 glyphs embedded but only 23 used — 98% waste").
- **Metadata stripping:** `strip_metadata(path, output_path)` removes XMP streams and Info fields (author/creator/producer); **preserve PDF/X-required keys** (OutputIntents, GTS_PDFXVersion).
- **`compress_pdf(path, settings, output_path) -> CompressionResult`** (original/compressed bytes, reduction_percent, images_downsampled, streams_recompressed, fonts_subsetted). **`CompressionPanel`:** analyze-first (estimated savings), settings sliders (target DPI, JPEG quality, compress streams), progress, before/after size comparison.

**Done when:** a 30 MB print PDF compresses under 10 MB with standard settings, and a font-heavy file shrinks measurably with subsetted fonts still rendering in Acrobat.

### 5.2 — Barcode detection & validation (#48)

- **`zxingcpp-rs`** (ZXing-C++ v2 bindings; verify Win/macOS build). `detect_barcodes(path, page_index) -> Vec<BarcodeDetection>` — render the page at 200 DPI via PDFium, decode (format, text, position, orientation).
- **Quiet-zone validation** per side: Code128 10× narrow-bar, QR 4× module, EAN-13 3.63 mm minimum — expand the bbox, check for non-white pixels in the quiet-zone area. `BarcodeValidation` (format, decoded, position, quiet_zone_{left,right,top,bottom}_ok, severity).
- **Size & decodability:** EAN-13 ≥ 26.73 × 18.28 mm (80% magnification); Code128 ≥ 19 mm tall (non-retail); QR ≥ 10 × 10 mm at 300 DPI. Display decoded content so the user can verify the encoded data.
- **`BarcodeCheck` UI:** run on all pages; table page | format | decoded | quiet zone | size | status; click → location overlay on the page render. Add to `CHECK_REGISTRY` and the General Print profile (off by default).

**Done when:** detected barcodes show decoded content with quiet-zone/size status, and undersized/tight-margin codes are flagged.

### 5.3 — Analytics dashboard (#50)

- **Aggregates** over `preflight_findings` / `batch_results`: pass rate by date (daily, last 30 days); top-10 errors by `check_name`; files processed per day (batch + manual); average file-size trend; average error count per file by client. `get_analytics(days) -> AnalyticsSummary`.
- **`AnalyticsDashboard`** (`recharts`): pass-rate line chart; top-errors horizontal bar; files-per-day bar; summary cards (files this month, pass rate, most common error).
- **Per-client view:** optional `client_id` on `pdf_jobs` (prompt "Associate with client?" on open, linking to the shop's `customers`/`jobs`); client filter dropdown; per-client pass rate, common errors, average turnaround.
- **Export:** analytics PDF (charts via canvas→PNG + `printpdf` layout) and raw `preflight_findings` CSV; date-range filter (this month / 30 / 90 / custom).
- **Privacy:** analytics derive only from local job metadata; never include file contents.

**Done when:** the dashboard renders accurate aggregates from real data and supports per-client filtering + export.

### 5.4 — Approval sheets & report export (#59, #60)

- **`generate_approval_sheet(path, job_info, output_path)`** (`printpdf = "0.9"`). `ApprovalSheetInfo` (client_name, job_number, due_date, description, staff_name, include_preflight_summary). A4 portrait: header/logo (from `business_info`), page-thumbnail grid (2×3, 150 DPI), job-info table, optional preflight summary (error/warning counts), sign-off lines ("Approved by: ____ Date: ____"). Options: brand colors, full findings list (multi-page), diagonal "PROOF" watermark. Wired into `ArtApprovalPanel` ("Generate Approval Sheet" per version, auto-filling job number from the order and client name from the linked client).
- **Report export:** `export_preflight_report_pdf(job_id, output_path)` (shop header, file info, run date, findings grouped by category Font/Page Boxes/Bleed/Color/PDF-X, pass/fail summary on page 1); `export_preflight_report_csv(job_id, output_path)` (columns check_name, severity, page_num, message, fix_hint); `export_preflight_report_json(job_id)` (full structured JSON shaped for MIS/webhook integration). "Export Report" dropdown (PDF / CSV / JSON) in `PreflightReport`.

**Done when:** an approval sheet generates from an order with logo + thumbnails, and reports export correctly in all three formats.

### 5.5 — AI visual checking & ink coverage (#45)

- **`check_ai_visual(path, page_index, api_key) -> AiVisualFinding`:** render at 150 DPI, base64 PNG, send to a vision model (Anthropic Claude API preferred) with a print-preflight system prompt ("examine for blur, pixelation, illegible text, unintended white areas, color banding, misaligned elements; list specific issues or say PASS"); parse into structured findings.
- **Batching:** "AI Visual Check" runs all pages with rate limiting (≤10/min) and a **pre-run cost estimate** (~1,000 tokens/page at 150 DPI); progress "Checking page N of N." Integrate as an "AI Visual Review" report section (page, issue, AI-assessed severity, quoted AI text, "Recheck this page") with a "supplementary — verify manually" disclaimer.
- **Tuning:** refine the prompt against real results (reduce false positives: "only report issues affecting print quality"; add categories: very small text < 6 pt, text touching the edge). Configurable shop-specific prompt prefix (e.g., "uncoated stock — flag tints below 10%"). Target < 2 false positives on a known-good file.
- **Privacy & consent (best practice):** AI visual is **off by default**; first use requires explicit consent that artwork is sent to a third-party API; API key stored in the OS keychain.
- **Ink coverage** (replaces the Phase 2 stub): `check_ink_coverage(path, max_tac)` — render at 72 DPI to CMYK-approximate, sample all pixels, compute average TAC, flag over threshold (300% coated / 260% uncoated) with an estimate caveat (doesn't fully account for overprint). Rich-black files report TAC near 300–400%.

**Done when:** AI checks return useful per-page findings within rate/cost limits behind explicit consent, and rich-black files report TAC near 300–400%.

---

## Phase 6 — Integration & Polish

*Issues: #54, #57, #58, #52 (partial) + final QA*

**Goal:** Outbound integrations (email, FTP, MIS webhook), keyboard shortcuts, in-app help, settings, and release readiness.

**Exit criteria:** A full real-shop workflow completes end-to-end; both installers run on clean Windows + macOS machines; performance baselines met.

### 6.1 — Email, FTP & MIS webhook (#54, #52)

- **Email** (`lettre = { version = "0.11", features = ["tokio1-native-tls"] }`): SMTP settings (host, port, username, password, use_tls, from_address) + `test_smtp_connection()`. `email_preflight_report(job_id, to_address, subject, body)` — generate the report PDF to temp, build + send with attachment; "Email Report" button → modal. "Send for Approval" in `ArtApprovalPanel` — generate the approval sheet, email the client's address, pre-filled subject ("Proof for Approval — [Order Number]") + customizable body template; log to the existing `invoice_reminders` table (method = email).
- **FTP** (`suppaftp = "5"`, pure-Rust): settings (host, port, username, password, base_path, use_ftps) + `test_ftp_connection()`; `upload_to_ftp(local_path, remote_path)`. Wire as a hot-folder output action — pass files → `base_path/pass/`, fail → `base_path/fail/` — with status in the activity log.
- **MIS webhook (#52 partial):** settings (URL, optional bearer token). `send_webhook(job_id, event_type)` POSTs JSON (job info + file name + preflight result summary + per-check results); events `preflight_complete` / `batch_complete` / `hot_folder_processed`; log delivery status + HTTP code. **Retry** with exponential backoff (1s/4s/16s) via a persisted `webhook_deliveries` queue (status pending/sent/failed) retried on startup and after each attempt. *(Mirrors the research backup-API discipline: idempotent, queued, retried — and is the natural seam for emitting the broader app's job lifecycle `events` to an MIS.)*
- **Security & testing (best practice):** all SMTP/FTP/webhook credentials in the OS keychain; outbound features require explicit enablement; validate certs, handle TLS/passive modes. Test SMTP against Gmail/Outlook/Mailgun-Sendgrid; FTP against FileZilla local + a remote server; webhook against webhook.site.

**Done when:** reports/approval sheets deliver by email, processed files upload to FTP, and webhooks deliver with reliable retry — all using securely stored credentials.

### 6.2 — Keyboard shortcuts & help system (#58, #57)

- **Global shortcuts** scoped to the PDF section (registered only when PDF is active, no conflict with text inputs): Cmd/Ctrl+O open; Cmd/Ctrl+R run preflight; Cmd/Ctrl+B batch; ←/→ page nav; +/− (or Cmd/Ctrl+=/−) zoom; Cmd/Ctrl+Shift+E export; Cmd/Ctrl+Shift+F find/replace; Esc close modal/deselect; `?` reference overlay. Per-button shortcut hints in tooltips; `?` overlay groups shortcuts by category.
- **Help system:** bundled markdown in `src-tauri/resources/help/` (`overview`, `font-embedding`, `image-dpi`, `color-spaces`, `bleed`, `pdfx`, `overprint`, `batch`, `hot-folders`, automation articles). `get_help_article(slug)`; `HelpPanel` (slide-out, searchable, categorized Preflight/Color/Automation/Integration, rendered markdown via `marked` w/ code highlighting). Contextual `?` icons next to each check map `check_id → article`; "Learn more" links in fix hints. Author **complete, non-technical** articles a print-shop employee can follow (font embedding steps in InDesign/Illustrator/Acrobat; CMYK vs RGB and why print needs CMYK; PDF/X-1a — what/who/guarantees; bleed — what/why/how; overprint — when correct vs error; profiles/action-lists/batch/hot-folders how-tos).
- **First-run onboarding overlay:** Step 1 open a PDF → Step 2 choose a profile → Step 3 run preflight + view results; Skip / Don't-show-again persisted (`has_seen_pdf_onboarding` in `business_info`).

**Done when:** every workflow is keyboard-operable with a reference overlay, and each check has a contextual help article a non-technical employee can follow.

### 6.3 — Settings, hardening & release readiness

- **`PdfSettings`** (`src/components/settings/PdfSettings.tsx`): default profile; default output dir; SMTP/FTP/webhook (with test buttons); AI key + token-usage display; temp-cleanup interval — all persisted; secrets via keychain.
- **Hardening & audits:** error-message audit (no raw debug strings; every error states cause + next action; inline banners for recoverable errors); accessibility pass (keyboard-only through PreflightReport/ProfileEditor/BatchPanel; icon+label not color alone; ARIA on icon-only buttons); memory audit (`cleanup_temp_files()` on close/new-open; release PDFium handles; < 500 MB after 10 large files in sequence); cross-platform verification (macOS Apple Silicon ARM64 PDFium + FSEvents + `/Users/...` paths; Windows 10 older hardware Core i5 4th-gen, NTFS hot-folder watcher, PDF file dialog).
- **QA & launch:** structured bug bash across every PDF component (fix P0/P1 immediately; document P2/P3) including concurrent batch + hot-folder stress and an action-list step that fails halfway (graceful partial output); **Phase-1 regression vs the golden baseline** (results unchanged after all later code); final perf baseline on reference hardware (open < 2 s, 20-page thumbnails < 5 s, 50-page preflight < 10 s, batch throughput documented); GitHub issue templates (bug, feature, **preflight-discrepancy** with PDF/X version + our finding + Acrobat finding + sample path) in `.github/ISSUE_TEMPLATE/`; user-facing feature descriptions / release notes (double as onboarding + marketing copy); final `cargo check` / `cargo clippy -- -D warnings` / `npx tsc --noEmit` clean; signed + notarized `cargo tauri build` installers verified on clean Windows + macOS machines (PDFium bundled, app starts with no dev tools); auto-updater verified against signed artifacts.

**Done when:** all settings persist with secrets in the keychain; audits pass; the Phase-1 regression matches the golden baseline; signed installers launch cleanly on fresh Windows + macOS; performance baselines met.

---

## Quick Reference

| Phase | Workstreams | Issues |
| --- | --- | --- |
| 1 — Preflight Foundation | Ingestion & viewer ✅; font embedding ✅; page boxes ✅; image DPI ✅; bleed + fixup; PDF/X compliance, inspector & findings store; integration & golden baseline | #21, #22, #24, #27, #28, #29, #30 |
| 2 — Color Detection & Conversion | Color space detection; overprint/transparency/hidden content; RGB→CMYK + ICC; Make-PDF/X-4 wizard (X-1a fallback) & spot/ink inventory | #23, #25, #26, #34 |
| 3 — Viewing & Editing | Viewer upgrades & inspection; layers & page ops; content-stream round-trip; text search/replace; image replace/optimize | #31, #32, #35, #36, #55, #56 |
| 4 — Automation Engine | Profiles & registries; action lists; batch; debugger; hot folders; integration | #38, #39, #40, #41, #42 |
| 5 — Advanced Features | Compression & font subsetting; barcodes; analytics; approval sheets & report export; AI visual & ink coverage | #45, #48, #49, #50, #59, #60 |
| 6 — Integration & Polish | Email/FTP/webhook; shortcuts & help; settings, hardening & release | #54, #57, #58, #52 |
| Cross-cutting (all phases) | Testing & golden corpus; CI/CD + signing/notarization; auto-update; observability & crash reporting; security & secrets; migrations & backup; privacy/consent; licensing & flags; perf/a11y/i18n | — |
| Product context (research) | Local-first job/intake model; multi-tenant SQLite + event log; job ticket generation; backup-only cloud API; estimating roadmap | — |
