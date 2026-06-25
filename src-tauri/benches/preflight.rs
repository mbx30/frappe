//! Benchmarks for `check_full_preflight` on a 20-page PDF (#298).
//!
//! Run with: `cargo bench --bench preflight`
//!
//! Measures the end-to-end preflight time (fonts + page boxes +
//! images + bleed + output intents + security + pdfx + color spaces +
//! overprint + transparency + hidden content). The metric of interest
//! is the per-page latency at p50 / p99.

use criterion::{criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

mod fixtures;

fn bench_full_preflight(c: &mut Criterion) {
    let input: PathBuf = fixtures::twenty_page_pdf();
    let path = input.to_str().unwrap().to_string();
    let mut group = c.benchmark_group("preflight");
    group.bench_function("twenty_page_full", |b| {
        b.iter(|| {
            // Use the path-based public API so this bench does not import
            // lopdf::Document directly. A direct import would create a second
            // compilation instance of lopdf when app_lib is built as cdylib,
            // causing an E0308 type mismatch at compile time.
            let _ = app_lib::preflight_cmds::check_full_preflight(path.clone());
        });
    });
    group.finish();
}

criterion_group!(benches, bench_full_preflight);
criterion_main!(benches);
