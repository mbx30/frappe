# Frappe — Performance SLOs and Baselines

This document pins the performance targets the dashboard and PDF
viewer must hold to remain responsive on the supported platforms.
SLOs are enforced in CI by `.github/workflows/perf.yml`; a >25 %
regression against the `main` baseline fails the workflow.

## SLOs

| Metric | Target (Windows) | Target (macOS) | Notes |
| --- | --- | --- | --- |
| Cold-start to interactive window | < 3.0 s | < 2.0 s | measured from process spawn to the first `window-ready` event |
| Dashboard orders list load (1k rows) | < 1.0 s | < 1.0 s | `list_orders` round-trip + render |
| DB query (any single-row command) | < 500 ms | < 500 ms | measured at the `Mutex<Connection>` lock acquisition + execution |
| RSS after 1 hour of idle dashboard | < 300 MB | < 250 MB | `metrics::sample_memory()` |
| `check_full_preflight` on a 20-page PDF | < 3.0 s | < 3.0 s | see `benches/preflight.rs` |
| `compress_pdf` on a 5 MB PDF (default opts) | < 2.0 s | < 2.0 s | see `benches/compression.rs` |
| `render_page` at 144 DPI | < 150 ms | < 120 ms | PDFium is the bottleneck on Apple Silicon |
| `render_page_b64` (same input) | < 200 ms | < 160 ms | adds PNG encode + base64 |

## Measurement methodology

- **Cold-start**: `tracing::info!("cold-start recorded")` in
  `lib.rs:metrics::record_cold_start` fires when the runtime is
  ready. The `MetricsSnapshot::cold_start_ms` field is what CI reads.
- **DB query latency**: `instrument_command!` macro in
  `metrics::instrument_command` records a per-command wall-time
  histogram. Anything above 1 s is logged as a slow op.
- **Memory**: `metrics::sample_memory()` reads the platform-specific
  RSS and stores it in `RESIDENT_BYTES`. CI samples it once per
  minute of the bench run.
- **Preflight / compression**: Criterion (issue #298). Output
  reports per-iteration wall-time and throughput.

## CI gate

`.github/workflows/perf.yml` runs the Criterion benches on every
push to `main` and on every PR. The workflow:

1. Restores a cached `target/bench-baseline/` from the previous
   `main` run.
2. Runs `cargo bench -- --save-baseline main` and the explicit
   `criterion` `bench.sh` driver.
3. Compares the per-bench medians against the baseline using
   `critcmp main pr` (or the in-house equivalent shipped in
   `scripts/perf.sh`).
4. Fails the workflow if any bench regressed by > 25 %.

To re-run locally:

```bash
# capture a new baseline
cargo bench -- --save-baseline my-run

# compare against the captured baseline
cargo bench -- --baseline my-run
```

## Comments / footnotes

- These are *SLOs*, not SLA guarantees. The CI gate is intentionally
  permissive (25 % regression budget) so that a noisy Linux
  runner cannot block a merge for an unrelated reason.
- The `compress_pdf` SLO assumes `CompressionOptions::default()`
  (`use_zopfli = false`). The opt-in Zopfli flag bypasses the
  SLO because its whole point is to take minutes for a 5 % size
  win; it is measured separately.
- Memory SLO is measured as RSS, not commit-charge. On macOS this
  excludes Metal-shared pages.
