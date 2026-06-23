# Issue #256 — Performance Metrics, Slow-Op Logging, and PerfOverlay

**Goal:** Collect and display backend performance metrics.

**Design sketch:**
- `tracing` spans for Tauri commands and DB queries.
- Emit `slow_op` events for operations >1s.
- `MetricsSnapshot` command and `PerfOverlay.tsx` UI.

**Prerequisites:**
- `tracing` is already wired.

**Implementation checklist:**
- [ ] metrics.rs module with command histograms.
- [ ] Slow-op logging.
- [ ] PerfOverlay UI toggle from settings.
