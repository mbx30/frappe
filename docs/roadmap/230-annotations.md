# Issue #230 — PDF Annotation Tools (Highlighting, Notes)

**Goal:** Allow users to highlight text and add sticky notes to PDFs.

**Design sketch:**
- Store annotations in a separate SQLite table keyed by PDF job id and page.
- Support highlight, underline, strike-through, freehand, sticky note.
- Render annotations as an overlay on top of the PDF page bitmap.
- Persist annotations and show page indicators.

**Prerequisites:**
- Stable page coordinate mapping from rendered bitmap back to PDF user space.
- Annotation model designed for future import/export.

**Implementation checklist:**
- [ ] Annotation model (page, rect, type, color, text, author, created_at).
- [ ] Annotation rendering overlay in PDFView.
- [ ] Toolbar for selecting annotation tool.
- [ ] Edit/delete annotations.
