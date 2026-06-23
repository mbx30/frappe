# Issue #282 — Windows+macOS CI Matrix, Signing, Notarization, Updater

**Goal:** Production-ready release pipeline.

**Design sketch:**
- CI builds Windows + macOS on every PR.
- Code-signed Windows installers (Authenticode).
- Notarized macOS apps.
- Tauri auto-updater wired to signed artifacts.

**Prerequisites:**
- Code-signing certificate and Apple Developer account.
- Release artifact hosting.

**Implementation checklist:**
- [ ] Extend ci.yml matrix.
- [ ] Create release.yml for signing/notarization.
- [ ] Configure updater public key.
