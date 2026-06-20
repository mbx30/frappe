# Frappe — Print Production Management & PDF Tooling

**Frappe** is a full-stack desktop application for print shops, creative studios, and production teams. Manage invoices, estimates, orders, and inventory—then automate PDF preflight, color conversion, compression, and editing workflows.

Built with **React 19 + TypeScript** (frontend) and **Rust + Tauri v2** (backend), with **SQLite** for persistent data.

## Product Context — Local-First Print Ops

Frappe is built as a **local-first** print-shop operations app: the desktop machine is the source of truth, SQLite holds all data, and the cloud is **backup-only in v1** (true sync arrives in V2). The schema is **multi-tenant from day one** — every table carries `tenant_id` — so the same codebase scales from a single-location pilot to multi-tenant SaaS without re-architecture.

- **ICP:** single-location digital print shops working paper-ish materials up to **13×19**.
- **Intake channels:** email (paste), walk-in, phone — captured into a fast, validated job ticket.
- **Job lifecycle:** `new → waiting_on_customer → ready_for_production → printing → finishing → ready_for_pickup_or_ship → completed`.
- **Color model (job level):** B/W or Color. **Proofing:** optional, not gating in v1.
- **Positioning wedge:** "Fastest, cleanest intake → idiot-proof job ticket → production board" for small shops; expand into estimating and true multi-tenant SaaS after workflow adoption.
- **Audit trail:** an append-only `events` log records the full row after every change, powering deterministic cloud backup and a future sync engine.

The PDF tooling operates on artwork attached to these jobs, so preflight results, fixups, and outbound actions tie back to clients and orders.

## Features

### Business Management
- **Invoices & Estimates** — Create, track, and manage pricing documents with line items, tax, and payment terms
- **Orders** — Full order lifecycle: prepress → production → shipping
- **Client Management** — Store client details, contact info, and order history
- **Inventory** — Track materials, components, and consumables with alerts and adjustments

### PDF Tools (Phase 1–5 Roadmap)

#### Phase 1: Preflight Foundation ✓ Complete
- **Font Checking** — Detect embedded vs. unembedded fonts, flag subsetting issues
- **Page Box Validation** — Verify MediaBox, TrimBox, BleedBox, ArtBox consistency
- **Image Resolution Analysis** — DPI checks, pixel dimensions, color space detection
- **Bleed Detection & Fixup** — Auto-add bleed to files, visual page box diagrams
- **PDF/X Compliance** — X-1a, X-3, X-4 validation with detailed findings

#### Phase 2: Color Space Detection & Conversion (In Progress)
- **Color Space Audits** — Identify CMYK, RGB, Lab, ICC-based color usage
- **Overprint & Transparency Detection** — Catch blend modes and opacity issues
- **RGB→CMYK Conversion** — Batch color conversion with lcms2 and ICC profiles
- **Hidden Content Detection** — Find off-page objects, default-off layers, white-on-white text
- **Spot Color Inventory** — List all PANTONE/custom spot colors with page usage

#### Phase 3: Viewing & Editing (Planned)
- Full-screen PDF viewer with zoom, navigation, page thumbnails
- Text search & replacement
- Image replacement & optimization
- Page operations (extract, delete, reorder, rotate)
- Layer visibility toggles

#### Phase 4: Automation Engine (Planned)
- **Preflight Profiles** — Custom check configurations with automatic fixes
- **Action Lists** — Record & replay PDF operations (add bleed, convert colors, etc.)
- **Batch Processing** — Process folders of PDFs with routing to pass/fail folders
- **Hot Folder Automation** — Real-time folder monitoring with auto-processing
- **Action List Debugger** — Step through operations with before/after page views

#### Phase 5: Advanced Features (Planned)
- **PDF Compression & Font Subsetting** — Downsample images, re-compress streams, strip metadata, remove unused glyphs
- **AI-Powered Visual Checking** — Vision-model preflight (opt-in, off by default) with pre-run cost estimate
- **Barcode Detection & Validation** — Decode + validate quiet zones and minimum sizes
- **Analytics Dashboard** — Pass rates, top errors, per-client trends
- **Approval Sheets & Report Export** — Branded sign-off sheets; PDF / CSV / JSON report export
- **Ink Coverage (TAC)** — Total-area-coverage estimate against coated/uncoated thresholds

#### Phase 6: Integration & Polish (Planned)
- **Email / FTP / MIS Webhook** — Send reports & approval sheets, upload outputs, push job events (with retry queue)
- **Keyboard Shortcuts & Help** — Full keyboard operability + non-technical in-app help articles
- **Settings, Hardening & Release** — Signed/notarized installers, auto-update, accessibility & memory audits

### Cross-Cutting Engineering Standards
Applied to every phase as part of "done": golden-file test corpus + CI regression gate + fuzzing; signed/notarized CI builds with auto-update; structured logging, crash reporting, and opt-in telemetry; secrets in the OS keychain; ordered idempotent migrations with `schema_version` and DB backup/restore; explicit per-feature consent for anything leaving the device; performance budgets (open < 2s, 20-page thumbnails < 5s, 50-page preflight < 10s), keyboard accessibility, and i18n-ready strings.

## Tech Stack

- **Frontend:** React 19, TypeScript, Vite, Tauri IPC
- **Backend:** Rust, Tauri v2, rusqlite
- **Database:** SQLite with WAL mode
- **PDF Libraries:** pdfium-render, lopdf, printpdf
- **Color:** lcms2 (ICC profile transformations)
- **Platform:** Windows 11, macOS 12.0+ (Apple Silicon)

## Build & Development

### Prerequisites
- Node.js 18+
- Rust 1.70+
- Tauri CLI: `npm install -g @tauri-apps/cli`

### Setup
```bash
git clone https://github.com/mbx30/frappe
cd frappe
npm install
npm run dev       # Start frontend + Tauri dev server
```

### Type Checking
```bash
cargo check       # Rust
npx tsc --noEmit  # TypeScript
```

### Build
```bash
npm run build     # Production bundle
cargo build --release
```

## Project Structure

```
frappe/
├── src/                    # React frontend
│   ├── components/         # UI components
│   │   ├── ManagementView.tsx     # Main dashboard
│   │   ├── InvoiceForm.tsx        # Invoice creation
│   │   └── preflight/             # PDF preflight checks
│   ├── design-system/      # Shared styles & components
│   ├── types.ts            # TypeScript models
│   └── App.tsx
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands.rs     # Tauri command handlers
│   │   ├── db.rs           # SQLite operations
│   │   ├── models.rs       # Data structures
│   │   ├── pdf/            # PDF processing modules
│   │   └── lib.rs          # Tauri app setup
│   └── Cargo.toml
├── PDF_TOOLING_PLAN.md     # Phase → workstream → task implementation roadmap
└── README.md               # This file
```

## Database Schema

All tables carry `tenant_id` for a clean path to multi-tenant SaaS.

### Business & Ops Tables
- `business_info` — Company details (onboarding state)
- `invoices`, `invoice_line_items` — Billing documents
- `estimates`, `estimate_line_items` — Quote documents
- `orders` — Production orders with status tracking
- `clients` / `customers` — Customer information
- `inventory` — Stock levels and item definitions

### Local-First Foundation (multi-tenant)
- `tenants`, `device` — Tenant and device registration
- `jobs` — Work orders (intake source, product type, qty, size, stock, color mode, promised date, status)
- `files` — Artwork/proof assets (path, mime, size, `sha256`)
- `events` — Append-only audit log (full row after change), drives cloud backup
- `backup_state` — Last uploaded sequence / snapshot tracking

### PDF Tooling Tables
- `pdf_jobs` — PDF file history and metadata (optional `client_id`)
- `preflight_findings` / `preflight_run_summary` — Check results and run history
- `preflight_profiles` / `profile_checks` / `profile_fixups` — Configurable profiles
- `action_lists` / `action_list_steps` — Recorded automation
- `batch_jobs` / `batch_results` — Batch runs
- `hot_folders` / `hot_folder_log` — Watched-folder automation
- `webhook_deliveries` — MIS webhook retry queue

## Key Commands

### Invoice/Estimate/Order Management
- `create_invoice(number, due_date, terms)` → Invoice
- `create_estimate(number, valid_until)` → Estimate
- `create_order(po_number, priority)` → Order
- `list_invoices()` → Vec\<Invoice\>

### PDF Preflight
- `check_fonts(path)` → Vec\<FontFinding\>
- `check_page_boxes(path)` → Vec\<PageBoxFinding\>
- `check_image_resolution(path, min_dpi)` → Vec\<ImageResolutionFinding\>
- `check_bleed(path, min_mm)` → Vec\<BleedFinding\>
- `check_pdfx(path, profile)` → Vec\<PdfXFinding\>

### PDF Fixups
- `add_bleed(path, amount_mm, output)` → ()
- `convert_rgb_to_cmyk(path, src_profile, dst_profile, output)` → ()
- `compress_pdf(path, settings, output)` → CompressionResult

## Roadmap

See [PDF_TOOLING_PLAN.md](./PDF_TOOLING_PLAN.md) for the full **phase → workstream → task** roadmap. Work is gated by explicit *Done when* acceptance criteria rather than a calendar; each workstream is proven, PR'd, and merged before the next begins.

**Current Status:** Phase 1 foundation complete (ingestion/viewer, fonts, page boxes, image DPI); remaining Phase 1 (bleed fixup, PDF/X, inspector, findings store) and Phase 2 (color detection & conversion) in progress.

## Issues & Testing

Open issues are organized by feature area:
- **#22–#30** — Phase 1: Preflight foundation
- **#23, #25, #26, #34** — Phase 2: Color detection & conversion
- **#31, #32, #35, #36, #55, #56** — Phase 3: PDF viewing & editing
- **#38–#42** — Phase 4: Automation engine
- **#45, #48, #49, #50, #59, #60** — Phase 5: Advanced features
- **#52, #54, #57, #58** — Phase 6: Integration & polish

Cross-cutting and product-foundation work (local-first job model, event log, cloud backup, golden corpus, CI signing, observability, secrets, migrations) is tracked in newer issues — see the roadmap's cross-cutting table.

For bug reports, see [#73](https://github.com/mbx30/frappe/issues/73) (bug hunt with child issues #78–#82).

## License

[MIT](./LICENSE) or specify your preference

## Contributing

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -am 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Contact

For questions or collaboration, open an issue or reach out to the maintainers.

---

**Frappe** — Powering print production workflows, one PDF at a time.
