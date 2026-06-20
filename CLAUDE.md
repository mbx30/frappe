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
- **Platform:** Windows 11 + macOS 12.0+ (Apple Silicon)

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
