# Issue #235 — PDF Reflowable Text and Reflow Display

**Goal:** Allow text layout to adapt when users increase text size.

**Design sketch:**
- Extract text and structure.
- Render a reflow view that wraps text at the viewport width.
- Preserve reading order and images.

**Prerequisites:**
- Structure tags (#233) and text extraction.
- Tag-based layout engine.

**Implementation checklist:**
- [ ] Text extraction pipeline.
- [ ] Reflow renderer component.
- [ ] Zoom without horizontal scroll.
