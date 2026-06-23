# Issue #234 — Alternative Text for Images and Graphics

**Goal:** Attach alt-text to images and graphics.

**Design sketch:**
- Right-click an image in PDFView to set `/Alt`.
- Distinguish decorative vs content-bearing images.
- Validate alt-text presence in accessibility checker.

**Prerequisites:**
- Image hit-testing (#263).
- Structure tags (#233) for proper association.

**Implementation checklist:**
- [ ] Alt-text model.
- [ ] Context menu in viewer.
- [ ] Write `/Alt` entries to XObjects.
- [ ] Accessibility checker integration.
