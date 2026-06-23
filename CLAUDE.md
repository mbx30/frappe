# Frappe — Claude Code Project Config

## Ignore These Paths
Claude should not read or index these directories:
- `src-tauri/target/` — Rust build artifacts (hundreds of auto-generated .toml files)
- `node_modules/` — npm packages
- `dist/` — production build output

## Stack
- **Frontend:** React 19 + TypeScript + Vite
- **Backend:** Rust + Tauri v2
- **Database:** SQLite (bundled via rusqlite)
- **Platform:** Windows 10+ (WebView2 required) + macOS 12.0+ (Apple Silicon) + Linux (modern glibc, or musl for old distributions)

## Key Conventions
- Tauri commands are in `src-tauri/src/commands.rs`, registered in `lib.rs`
- DB methods are in `src-tauri/src/db.rs`
- Models (Rust structs) are in `src-tauri/src/models.rs`
- Frontend types mirror Rust models in `src/types.ts`
- All components use the design system from `src/design-system/`
- Dates stored as `YYYY-MM-DD` strings; compare as strings, not `new Date()`

## Build
```
npm run dev       # Start frontend + Tauri dev
cargo check       # Rust type check (run from src-tauri/)
npx tsc --noEmit  # TypeScript type check
```

## Rust Code Quality (CodeQL Notes)

CodeQL's Rust analysis reports "Low analysis quality" for this codebase — this is **expected**, not a defect. The metrics that matter:

- **Call target resolution: 46%** (threshold 50%) — low because the codebase uses 150+ `#[tauri::command]` procedural macros plus heavy `#[derive(...)]` usage. CodeQL cannot trace through macro-expanded code, so it under-estimates reachable call targets. The 46% number is not a quality regression — it reflects a tool limitation, not a code smell.
- **Expression type inference: 56%** (threshold 20%) — well above threshold. Real type-safety is fine.

The codebase has zero `unsafe` blocks, no format-string injection vectors, no null-byte path-traversal windows (path validation enforces this), and uses `Result`-based error handling throughout.

### How to write Rust here so static analysis (and humans) can follow it

- **Keep business logic in plain Rust functions, not behind macro invocations.** A `#[tauri::command]` handler should parse args, call into a testable function in `src-tauri/src/pdf/`, `db.rs`, etc., then map the result. Don't bury real logic inside the macro body.
- **Prefer explicit return types on public functions.** Aids inference and makes the contract obvious to readers scanning the file.
- **Use `where T: Trait` clauses or fully-named types over `impl Trait` in public APIs** when a future caller might want to name the type.
- **Group related `#[tauri::command]` handlers in `commands.rs`** with a one-line doc comment each. The macro overhead is concentrated; the actual logic is testable.
- **When adding a new `#[derive(...)]`**, only add derives that the struct actually uses (e.g. don't add `Serialize`/`Deserialize` to internal-only types). Extra derives add macro-expansion surface for nothing.
- **Add a one-line rustdoc on every public command** explaining what it does, what it touches, and what the error cases are. This helps human reviewers and partially offsets the analysis gap.

### Build / lint / format

Run from `src-tauri/`:

```
cargo check             # type check
cargo clippy --lib      # lint (Windows requires GTK dev headers for full run; --lib is the pure-Rust path)
cargo fmt               # format
```

If `cargo check` is clean, the code compiles — that is the source of truth for "does it build." CodeQL's "low analysis quality" banner is a tool reporting limitation, not a build failure.
