# Frappe — Print Production Management & PDF Tooling

**Frappe** is a full-stack desktop application for print shops, creative studios, and production teams. Manage invoices, estimates, orders, and inventory—then automate PDF preflight, color conversion, compression, and editing workflows.

Built with **React 19 + TypeScript** (frontend) and **Rust + Tauri v2** (backend), with **SQLite** for persistent data.

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
- **PDF Compression** — Downsample images, re-compress streams, remove metadata
- **AI-Powered Visual Checking** — Detect low-resolution images, text overflow
- **Barcode Detection & Validation** — Find and validate barcodes
- **Dieline Derivation** — Auto-generate cut lines for labels and packaging

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
├── PDF_TOOLING_PLAN.md     # 270-day implementation roadmap
└── README.md               # This file
```

## Database Schema

### Core Tables
- `business_info` — Company details (onboarding state)
- `invoices`, `invoice_line_items` — Billing documents
- `estimates`, `estimate_line_items` — Quote documents
- `orders` — Production orders with status tracking
- `clients` — Customer information
- `inventory` — Stock levels and item definitions
- `pdf_jobs` — PDF file history and metadata
- `preflight_findings` — Test results from PDF checks

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

See [PDF_TOOLING_PLAN.md](./PDF_TOOLING_PLAN.md) for the full 270-day implementation schedule across 6 phases.

**Current Status:** Phase 1 complete (preflight foundation), Phase 2 in progress (color space detection).

## Issues & Testing

Open issues are organized by feature area:
- **#22–#30** — Phase 1: Preflight foundation
- **#31–#37** — Phase 3: PDF viewing & editing
- **#38–#42** — Phase 4: Automation engine
- **#43–#60+** — Phase 5: Advanced features

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
