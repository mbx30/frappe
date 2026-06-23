# Frappe — Platform Compatibility

## Supported Targets

| OS | Minimum version | Status | Notes |
|----|-----------------|--------|-------|
| Windows 10 | 1809 (build 17763) | ✓ Supported | WebView2 Runtime required (preinstalled on Win11; one-time install on Win10) |
| Windows 11 | All builds | ✓ Supported (recommended) | WebView2 Runtime preinstalled |
| macOS | 12.0 Monterey+ | ✓ Supported | Apple Silicon native; Rosetta fallback works on Intel |
| Linux (modern) | glibc 2.31+ (Ubuntu 20.04+, Debian 11+, etc.) | ✓ Supported | Use the standard Tauri bundle (`.deb` / `.AppImage` / `.rpm`) |
| Linux (legacy) | Any 2.6.23+ kernel (2007+) | ✓ Supported | Use the musl static build — see `build-linux-musl.sh` and `Dockerfile.legacy` |

## Not Supported

| OS | Reason |
|----|--------|
| Windows XP / Vista / 7 / 8 / 8.1 | Tauri v2 uses Microsoft Edge WebView2 for the webview layer. WebView2 has no presence on these releases; the binary will not start. |
| Linux distributions without a 2.6.23+ kernel | Tauri's webkit2gtk dependency requires a modern kernel. |
| 32-bit Windows | Tauri v2 builds only target x86_64 Windows. |

## Key Build Artifacts

| Build | Command | Output |
|-------|---------|--------|
| Windows 10+ | `build-windows.bat` or `npm run tauri build` | `src-tauri/target/x86_64-pc-windows-msvc/release/frappe.exe` + MSI/NSIS installers |
| macOS 12+ | `npm run tauri build` | `.app` bundle + `.dmg` |
| Linux (modern) | `npm run tauri build` | `.deb` / `.AppImage` / `.rpm` |
| Linux (legacy / musl) | `build-linux-musl.sh` | `src-tauri/target/x86_64-unknown-linux-musl/release/libapp_lib.so` (static) |

## Runtime Dependencies

| Platform | Required runtime |
|----------|------------------|
| Windows 10+ | [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (Evergreen; auto-updates via Edge) |
| macOS 12+ | None (WebKit is built into the OS) |
| Linux (modern) | `webkit2gtk-4.1`, `libgtk-3`, `libayatana-appindicator3`, `librsvg2`, `libdbus-1` (install via your package manager) |
| Linux (musl static) | None — fully self-contained |

## CI Build Matrix

| Job | Runner | Produces |
|-----|--------|----------|
| `frontend` | ubuntu-latest | TypeScript check, ESLint, `dist/` artifact |
| `rust` | ubuntu + windows + macos | `cargo check`, `cargo clippy`, `cargo test`, `cargo fmt --check` |
| `docker` | ubuntu-latest | `frappe:<sha>` Docker image (Debian Bookworm runtime) |
| `legacy-binaries` | ubuntu-latest | Static musl Linux `libapp_lib.so` via `Dockerfile.legacy` |

## Version Matrix

| Component | Version | Notes |
|-----------|---------|-------|
| Rust | 1.77.2 | MSRV; do not bump without testing the full dependency tree |
| Node.js | 22 LTS | Required by Vite 8 + Tauri CLI 2.11 |
| Tauri | 2.11.2 | WebView2-based on Windows; webkit2gtk on Linux |
| React | 19 | |
| TypeScript | 6.0 (or 5.x in transit) | See `tsconfig.json` for the active target |

## Known Limitations

- **Windows WebView2 dependency**: Win10 1809 users must install the WebView2 Evergreen Runtime if it is not preinstalled (Windows 11 and most modern Win10 machines have it via Edge).
- **Linux webkit2gtk version**: Building Tauri on Linux requires `webkit2gtk-4.1` (not the older `-4.0`). Distributions older than Ubuntu 20.04 / Debian 11 do not ship this version.
- **macOS 12.0 minimum**: Required by Tauri 2.11's `WebKit.framework` minimum. macOS 11 Big Sur and earlier are not supported.
- **No 32-bit Windows support**: Tauri v2 dropped 32-bit Windows targets. For legacy 32-bit Windows you would need a different webview layer (out of scope).
