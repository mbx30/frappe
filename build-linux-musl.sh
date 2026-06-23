#!/bin/bash
# Build script for Linux: produces a fully static musl binary that runs
# on any Linux distribution (kernel 2.6.23+, glibc 2007+). Useful for
# older systems and air-gapped deployments.
#
# The release `.deb` / `.AppImage` / `.rpm` artifacts produced by
# `npm run tauri build` target modern glibc systems. This script
# instead cross-compiles with the musl target to give you one binary
# that runs everywhere.
#
# Run from the project root on any Linux machine with Rust + Node 22+.

set -e

cd src-tauri

echo "Building for Linux (x86_64, musl static)..."
rustup target add x86_64-unknown-linux-musl 2>/dev/null || true

RUSTFLAGS="-C target-feature=+crt-static" \
  cargo build --release \
    --lib \
    --target x86_64-unknown-linux-musl \
    2>&1 | tee build.log

echo "Build complete. Binaries in target/x86_64-unknown-linux-musl/release/"
ls -la target/x86_64-unknown-linux-musl/release/*.{so,d} 2>/dev/null || echo "No shared libs found"

echo
echo "Note: to build the full Tauri app (not just the lib), use:"
echo "  npm run tauri build -- --target x86_64-unknown-linux-musl"
