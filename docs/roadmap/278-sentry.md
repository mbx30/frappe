# Issue #278 — Crash Reporting, Telemetry, and Keychain Audit

**Goal:** Opt-in Sentry crash reporting and telemetry.

**Design sketch:**
- Integrate `sentry` crate behind explicit consent toggle.
- Capture Rust panics and frontend errors.
- Audit credentials are in OS keychain.

**Prerequisites:**
- Sentry DSN.
- Privacy policy defining what is collected.

**Implementation checklist:**
- [ ] Add `sentry` dependency.
- [ ] Consent UI in settings.
- [ ] Panic and error handlers.
