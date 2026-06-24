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
    let mut group = c.benchmark_group("preflight");
    group.bench_function("twenty_page_full", |b| {
        b.iter(|| {
            let doc = lopdf::Document::load(input.to_str().unwrap())
                .expect("load pdf");
            // Re-run every check the public command runs. The benchmark
            // measures the combined cost of the suite.
            let _pdfx = crate::pdf::pdfx::check_metadata(&doc);
            let _color = crate::pdf::color::check_color_spaces(&doc, "any");
            let _overprint = crate::pdf::overprint::check_overprint(&doc);
            let _transparency = crate::pdf::overprint::check_transparency(&doc);
            let _hidden = crate::pdf::overprint::check_hidden_content(&doc);
            let _fonts = crate::pdf::fonts::collect_fonts(&doc);
            let _boxes = crate::pdf::boxes::check_page_boxes(&doc);
            let _images = crate::pdf::images::check_image_resolution(&doc);
            let _bleed = crate::pdf::bleed::check_bleed(&doc, 3.0);
            let _oi = crate::pdf::metadata::get_output_intents(&doc);
            let _sec = crate::pdf::security::check_security(&doc);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_full_preflight);
criterion_main!(benches);
