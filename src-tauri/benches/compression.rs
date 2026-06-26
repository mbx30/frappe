//! Benchmarks for `compress_pdf` (#298).
//!
//! Run with: `cargo bench --bench compression`
//!
//! Generates a ~5 MB PDF on first run (cached in `target/bench-fixtures`)
//! and times the full compress pipeline. The metric of interest is
//! `compress_pdf` throughput in MB/s on a real-world-ish input.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::path::PathBuf;

mod fixtures;

fn bench_compress_pdf(c: &mut Criterion) {
    let input: PathBuf = fixtures::five_mb_pdf();
    let output = input.with_extension("compressed.pdf");
    let size = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
    let mut group = c.benchmark_group("compress_pdf");
    group.throughput(Throughput::Bytes(size));
    group.bench_function("five_mb_default", |b| {
        b.iter(|| {
            let opts = app_lib::pdf::compress::CompressionOptions::default();
            let res = app_lib::pdf::compress::compress_pdf(
                input.to_str().unwrap(),
                Some(output.to_str().unwrap()),
                &opts,
            );
            // Ignore the Result — Criterion just wants wall-time.
            let _ = res;
        });
    });
    group.finish();
}

criterion_group!(benches, bench_compress_pdf);
criterion_main!(benches);
