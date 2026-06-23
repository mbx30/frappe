# Issue #275 — PDF Settings Panel and Release-Readiness Audits

**Goal:** Central PDF settings panel and pre-release hardening.

**Design sketch:**
- `PdfSettings.tsx` with default profile, output dir, integrations, AI key.
- Error-message audit, accessibility pass, memory audit.
- GitHub issue templates, release notes.

**Prerequisites:**
- Integrations (#274), AI visual (#273), metrics (#256).

**Implementation checklist:**
- [ ] PdfSettings panel UI.
- [ ] Persistence and keychain storage.
- [ ] Error/accessibility/memory audits.
