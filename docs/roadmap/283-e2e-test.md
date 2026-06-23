# Issue #283 — End-to-End PDF Automation Integration Test

**Goal:** CI gate that runs profile → action list → batch → hot folder → debugger end-to-end.

**Design sketch:**
- Rust integration test creating a profile, recording an action list, running batch, and watching hot folder.
- CI job `integration-pdf-automation`.

**Prerequisites:**
- Profiles (#265), action lists (#266), batch (#267), hot folder (#269), debugger (#268).

**Implementation checklist:**
- [ ] Create test fixture corpus.
- [ ] Write integration test.
- [ ] Add CI job.
