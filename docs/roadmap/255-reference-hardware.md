# Issue #255 — Cross-Platform Reference Hardware Verification

**Goal:** Verify the app runs on Windows 10 1809, Windows 11 RTM, macOS 12.0, and modern Linux.

**Design sketch:**
- Capture cold-start time, memory after 60s idle, and load test with heaviest PDF.
- Compare against SLOs in OPTIMIZATION.md.
- Publish results to `docs/perf-baseline.md`.

**Prerequisites:**
- Physical or VM access to reference hardware.
- CI matrix for smoke tests.

**Implementation checklist:**
- [ ] Run smoke tests on each target.
- [ ] Record startup, memory, and load metrics.
- [ ] Commit baseline report.
