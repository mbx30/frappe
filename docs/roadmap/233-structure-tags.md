# Issue #233 — PDF Document Structure Tags (Accessibility)

**Goal:** Add structure tags for proper reading order and PDF/UA compliance.

**Design sketch:**
- Build a structure tree from detected headings, paragraphs, lists, tables.
- Add `/StructTreeRoot` to catalog.
- Tag interactive and content elements.

**Prerequisites:**
- Content stream parsing for text layout.
- Tagged-PDF output validation.

**Implementation checklist:**
- [ ] Structure element model.
- [ ] Auto-tagging heuristic.
- [ ] Manual tag editor.
- [ ] Export tagged PDF.
