# Frappe

<p align="center">
  <img src="src/assets/hero.png" width="320" alt="Frappe — Print Shop Management & PDF Tooling" />
</p>

<p align="center">
  <strong>Local-first print-shop management + idiot-proof PDF preflight. Built for shops running paper-ish work up to 13×19.</strong>
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/github/license/mbx30/frappe?style=flat-square&color=blue" alt="License: MIT" /></a>
  <a href=".github/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/mbx30/frappe/ci.yml?style=flat-square&label=build" alt="Build status" /></a>
  <img src="https://img.shields.io/badge/React-19.2-149eca?style=flat-square&logo=react&logoColor=white" alt="React 19.2" />
  <img src="https://img.shields.io/badge/Tauri-v2.11-FFC131?style=flat-square&logo=tauri&logoColor=white" alt="Tauri v2.11" />
  <img src="https://img.shields.io/badge/Rust-1.77%2B-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust 1.77+" />
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey?style=flat-square" alt="Platform: Windows | macOS | Linux" />
</p>

Frappe is a desktop app for single-location digital print shops. It runs the business end — **invoices, estimates, orders, clients, inventory, payments, art approvals, job tickets, kanban board** — and the production end — **PDF preflight, color auditing, page-box validation, and (soon) edit and automation** — in one Tauri shell. Every byte lives on the local SQLite database; the cloud is **backup-only in v1**.

If you've ever hand-built a job ticket in Word, fought with a customer's RGB PDF, or stitched together an order tracker in a spreadsheet, Frappe is for you.

---

## 🚀 Get Started

Get a working dev build in under 5 minutes. Pick the path that matches your setup:

### Fastest — Windows / macOS / Linux dev host

```bash
git clone https://github.com/mbx30/frappe
cd frappe
npm install
npm run dev
```

The Tauri dev window opens automatically. First run pulls Rust crates and pdfium, so give it a few minutes.

### Most flexible — system prerequisites locked down

See **[BUILD.md](./BUILD.md)** for the per-platform dependency list. Quick reference:

| Platform | One-time install |
|----------|------------------|
| Linux (Ubuntu 22.04+ / Debian 12+) | `apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev pkg-config build-essential libsoup-3.0-dev` |
| macOS 12+ | `xcode-select --install` |
| Windows 10/11 | Visual Studio 2022 Build Tools with the *Desktop development with C++* workload |

> **Tip:** Frappe uses `rustls-tls` (not OpenSSL), so you can skip `libssl-dev` on Linux — fewer system packages, faster CI.

### Headless / container build

```bash
docker run --rm -v "$PWD:/app" rust:bookworm bash -c \
  "apt-get update && apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
   libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev pkg-config \
   build-essential libsoup-3.0-dev && cd /app/src-tauri && cargo check"
```

---

## Why Frappe?

**A print shop is not a generic ERP, and a print-ready PDF is not a generic document.** Frappe treats both as first-class.

- **14 dedicated PDF modules** (`bleed`, `boxes`, `color`, `content_stream`, `engine`, `fonts`, `images`, `metadata`, `overprint`, `pdfx`, `security`, `ticket`, `transforms`, `mod`) — not a wrapper around a generic PDF lib.
- **6-phase PDF tooling roadmap** — Phase 1 (preflight foundation) ✅ shipped, Phase 2 (color) 🚧 in progress, Phases 3-6 📋 planned. Full workstream breakdown in [PDF_TOOLING_PLAN.md](./PDF_TOOLING_PLAN.md).
- **Multi-tenant schema from day one** — every table carries `tenant_id`, so the same code base scales from a single-shop pilot to a multi-tenant SaaS without a rewrite.
- **Append-only event log** — the full row is recorded after every change. This powers deterministic cloud backup *and* a future bidirectional sync engine.
- **Local-first by design** — SQLite is the source of truth, the desktop is the only thing that has to be online. Cloud is backup-only in v1.
- **Opt-in at-rest encryption** — SQLCipher is wired behind a Cargo feature flag. Default build uses plain SQLite; opt in with `cargo build --features sqlcipher`.
- **Zero OpenSSL dependency** — `reqwest` is pinned to `rustls-tls`. No `libssl-dev` on Linux, no OpenSSL surprises on Windows.
- **Cross-platform CI matrix** — GitHub Actions builds run on `ubuntu-latest`, `windows-latest`, and `macos-latest` in parallel on every PR.

---

## 📚 Features

### 🧾 Business Operations

- **Invoices & Estimates** — line items, tax, payment terms, valid-until dates
- **Orders** — full lifecycle tracking, status machine, priority
- **Clients** — contact info, order history, art approval state
- **Inventory** — stock levels, item definitions, adjustment logs
- **Payments** — payment capture, status, reconciliation
- **Art Approvals** — per-job approval workflow with file history
- **Job Tickets** — printable shop-floor tickets with QR codes (via `qrcode` + `printpdf`)
- **Kanban Board** — drag-and-drop status transitions
- **Status machine** — `prepress → production → delivery → completed`
- **Spreadsheet View** — built on `react-data-grid` for fast bulk editing

### 📄 PDF Tooling

#### ✅ Phase 1 — Preflight Foundation (shipped)

- **Font checking** — detect embedded vs. unembedded fonts, flag subsetting issues
- **Page box validation** — verify `MediaBox`, `TrimBox`, `BleedBox`, `ArtBox` consistency with visual page-box diagrams
- **Image resolution analysis** — DPI checks, pixel dimensions, color-space detection
- **Bleed detection & fixup** — auto-add bleed to files missing it
- **PDF/X compliance** — `X-1a`, `X-3`, `X-4` validation with detailed findings

#### 🚧 Phase 2 — Color (in progress)

- **Color-space audits** — identify CMYK, RGB, Lab, ICC-based color usage
- **Overprint & transparency detection** — catch blend modes and opacity issues
- **RGB→CMYK conversion** — ICC-profile-driven batch conversion
- **Hidden content detection** — off-page objects, default-off layers, white-on-white text
- **Spot color inventory** — list every PANTONE / custom spot color with per-page usage

#### 📋 Phase 3 — Viewing & Editing (planned)

Full-screen PDF viewer (zoom, navigation, page thumbnails), text search & replace, image replacement & optimization, page operations (extract, delete, reorder, rotate), layer visibility toggles.

#### 📋 Phase 4 — Automation Engine (planned)

Configurable preflight profiles, record-and-replay action lists, batch processing with pass/fail routing, hot-folder automation with real-time folder monitoring, action-list debugger with before/after page views.

#### 📋 Phase 5 — Advanced (planned)

Compression & font subsetting, opt-in AI visual checking, barcode detection & validation, analytics dashboard, branded approval sheets, ink-coverage (TAC) estimation.

#### 📋 Phase 6 — Integration & Polish (planned)

Email / FTP / MIS webhook delivery with retry queue, full keyboard operability, signed/notarized installers with auto-update.

### 🔧 Cross-Cutting Engineering

Applied to every phase as part of "done": golden-file test corpus + CI regression gate, structured logging via `tracing`, secrets in the OS keychain via `keyring`, ordered idempotent migrations with `schema_version` and DB backup/restore, explicit per-feature consent for anything leaving the device, performance budgets (open < 2s, 20-page thumbnails < 5s, 50-page preflight < 10s), keyboard accessibility, and i18n-ready strings.

---

## 🏗️ Tech Stack

| Layer | Library / Tool | Version |
|-------|----------------|---------|
| **Frontend** | React | 19.2 |
| | TypeScript | 6.0 |
| | Vite | 8.0 |
| | react-data-grid | 7.0 |
| **Desktop shell** | Tauri | v2.11 |
| | Tauri plugins (dialog, log) | 2.x |
| **Backend** | Rust edition | 2021 (MSRV 1.77.2) |
| | tauri | 2.11 |
| | reqwest (rustls-tls) | 0.12 |
| | serde / serde_json | 1 |
| | tracing / tracing-subscriber | 0.1 / 0.3 |
| | keyring | 3 |
| | uuid (v4) | 1 |
| | chrono | 0.4 |
| | calamine + csv | 0.24 / 1.3 |
| **Database** | rusqlite (bundled, WAL) | 0.34 |
| | SQLCipher (opt-in feature flag) | — |
| **PDF — rendering** | pdfium-render | 0.9 |
| **PDF — editing** | lopdf | 0.41 |
| **PDF — generation** | printpdf | 0.7 |
| **PDF — images** | image, flate2 | 0.25 / 1.0 |
| **OS integration** | open, dirs, url | 5 / 6 / 2 |
| **Tooling** | ESLint, typescript-eslint, Vite plugin | latest |

---

## 🏛️ Architecture

### Local-first

The desktop is the source of truth. SQLite holds all data. The cloud is **backup-only in v1** — true bidirectional sync is V2. Your shop keeps working when the internet doesn't.

### Multi-tenant from day one

Every table — `invoices`, `clients`, `orders`, `pdf_jobs`, `preflight_findings`, all of them — carries a `tenant_id` column. The same code base serves a single-location pilot and a future multi-tenant SaaS without a re-architecture.

### Append-only event log

An `events` table records the full row after every change. This makes cloud backup deterministic (replay the log to rebuild state on another machine) and gives V2 sync a ready-made replication primitive.

### SQLCipher encryption (opt-in)

Default builds use plain SQLite — fast, zero-config, no host OpenSSL required. Operators who need at-rest encryption build with `cargo build --features sqlcipher`; the encryption code paths compile out cleanly otherwise.

### Tauri v2 IPC

The React frontend talks to the Rust backend through Tauri commands. Commands live in [`src-tauri/src/commands.rs`](./src-tauri/src/commands.rs) and are registered in [`src-tauri/src/lib.rs`](./src-tauri/src/lib.rs). TypeScript types in [`src/types.ts`](./src/types.ts) mirror the Rust models in [`src-tauri/src/models.rs`](./src-tauri/src/models.rs).

### Cross-platform

- **Windows 11** — Visual Studio 2022 Build Tools (MSVC), WebView2 runtime
- **macOS 12.0+** (Apple Silicon) — Xcode Command Line Tools
- **Linux** — Ubuntu 22.04+ / Debian 12+ with the apt packages listed in [BUILD.md](./BUILD.md)

### Frontend design system

All components draw from [`src/design-system/`](./src/design-system/) — shared tokens, components, guidelines, and UI kits. No ad-hoc styling in feature code.

---

## 🛠️ Build & Develop

### Type-check & build

```bash
npm run dev          # Vite + Tauri dev shell (hot reload)
npm run build        # Type-check + production frontend bundle
npx tsc -b --noEmit  # TypeScript only
```

From inside `src-tauri/`:

```bash
cargo check              # Rust type check
cargo build --release    # Production Rust binary
```

### Lint

```bash
npm run lint         # ESLint (typescript-eslint + react-hooks)
```

### Project layout

```
frappe/
├── src/                    # React 19 + TypeScript frontend
│   ├── components/         # Feature components (invoices, orders, preflight, ...)
│   ├── design-system/      # Tokens, UI kits, guidelines
│   ├── assets/             # hero.png, static assets
│   └── types.ts            # TS mirror of Rust models
├── src-tauri/              # Rust + Tauri v2 backend
│   └── src/
│       ├── commands.rs     # Tauri command handlers
│       ├── db.rs           # SQLite + migrations
│       ├── models.rs       # Rust structs
│       ├── pdf/            # 14 PDF modules
│       │   ├── bleed.rs  boxes.rs  color.rs  content_stream.rs
│       │   ├── engine.rs fonts.rs  images.rs metadata.rs
│       │   ├── mod.rs    overprint.rs pdfx.rs security.rs
│       │   └── ticket.rs transforms.rs
│       ├── cloud_backup.rs # Backup-only sync (v1)
│       ├── keychain.rs     # OS keychain integration
│       └── lib.rs          # Tauri app setup & command registration
├── .github/workflows/ci.yml   # Cross-platform CI matrix
├── PDF_TOOLING_PLAN.md        # Full 6-phase roadmap
├── BUILD.md                   # Per-platform build requirements
└── README.md
```

---

## 🗺️ Roadmap

The PDF tooling work is gated by explicit *Done when* acceptance criteria — each workstream is proven, PR'd, and merged before the next begins. See **[PDF_TOOLING_PLAN.md](./PDF_TOOLING_PLAN.md)** for the full **phase → workstream → task** breakdown.

| Phase | Scope | Status |
|-------|-------|--------|
| **1** — Preflight foundation | Ingestion, viewer, fonts, page boxes, image DPI, bleed, PDF/X, inspector, findings store | ✅ Shipped |
| **2** — Color | CMYK/RGB/Lab/ICC audits, overprint, hidden content, spot color inventory, RGB→CMYK conversion | 🚧 In progress |
| **3** — Viewing & editing | PDF viewer, text search/replace, image replace, page ops, layer toggles | 📋 Planned |
| **4** — Automation | Preflight profiles, action lists, batch processing, hot folders, action-list debugger | 📋 Planned |
| **5** — Advanced | Compression, font subsetting, AI visual checking, barcodes, analytics, approval sheets, ink coverage | 📋 Planned |
| **6** — Integration & polish | Email/FTP/MIS webhooks, keyboard shortcuts, signed installers, auto-update | 📋 Planned |

---

## ❓ FAQ

**Q: Is this production-ready?**
A: Phase 1 preflight (fonts, page boxes, image DPI, bleed, PDF/X) is shipped and gated by a CI regression suite. Phase 2 color work is in progress. Business ops (invoices, estimates, orders, clients, inventory, payments, kanban, job tickets) are functional. Treat it as **alpha-quality** for new shop rollouts today, and watch the [PDF_TOOLING_PLAN.md](./PDF_TOOLING_PLAN.md) for the Phase 2-6 milestones.

**Q: Does it run on Linux?**
A: Yes — Ubuntu 22.04+ and Debian 12+ are first-class targets. The CI matrix builds all three platforms on every PR. Install the apt packages listed in [BUILD.md](./BUILD.md) and you should be running in minutes.

**Q: How is my data stored?**
A: All data lives in a local SQLite database on the desktop. Default builds use plain SQLite with WAL mode. If you need at-rest encryption, build with the `sqlcipher` Cargo feature flag and supply an encryption key. Nothing leaves the machine in v1 except explicit, opt-in cloud backups.

**Q: Can I use it fully offline?**
A: Yes. Local-first means the desktop is the source of truth and no network call is required for the core workflow. Cloud backup (when you turn it on) and any future MIS integration are explicit, opt-in features.

**Q: Why not just use Acrobat?**
A: Acrobat is great for *fixing* a PDF; it doesn't tie that fix to the job it came from, the client who paid for it, the invoice it should land on, or the production board tracking it. Frappe binds the PDF to the business. Pre-flighted files move from intake to job ticket to kanban to delivery, with the preflight report attached the whole way.

**Q: Why Tauri and not Electron?**
A: Smaller binaries, lower memory, Rust's type system on the backend where the PDF and SQLite work lives, and `rustls-tls` keeps us off OpenSSL. The React frontend is unchanged from a typical web app.

**Q: Can I extend it?**
A: Yes. New Tauri commands go in `src-tauri/src/commands.rs` and are registered in `lib.rs`. New PDF checks are a module in `src-tauri/src/pdf/`. The frontend types mirror the Rust models, so `tsc` will catch contract drift.

---

## 🤝 Contributing

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/your-thing`)
3. Run `npm run lint`, `npx tsc -b --noEmit`, and `cargo check` before pushing
4. Commit (`git commit -am 'Add your-thing'`)
5. Push (`git push origin feature/your-thing`)
6. Open a Pull Request — the CI matrix will run on all three platforms

Bug reports and feature requests: **[open an issue](../../issues)**. For security issues, please open a private security advisory rather than a public issue.

---

## 📜 License

[MIT](./LICENSE) © Frappe contributors

---

## 📬 Contact

Issues, questions, and feature requests go to **[GitHub Issues](../../issues)**. The maintainers triage there first.
