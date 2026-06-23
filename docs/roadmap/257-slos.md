# Issue #257 — Encode Performance SLOs as CI Gates

**Goal:** Make SLOs enforceable in CI.

**Design sketch:**
- Document SLOs in `docs/perf-baseline.md`.
- Add lightweight CI job that fails on regression >25%.
- Surface compatibility matrix in BUILD.md.

**Prerequisites:**
- Metrics collection (#256).
- Reference hardware results (#255).

**Implementation checklist:**
- [ ] Commit SLO source of truth.
- [ ] Add regression-gate CI job.
