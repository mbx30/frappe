# Issue #231 — Redaction and Content Removal

**Goal:** Permanently remove sensitive information from PDFs.

**Design sketch:**
- Draw redaction rectangles over content.
- On apply, remove the underlying content stream operators and replace with black rectangles.
- Remove metadata that could reveal redacted content.
- Generate an audit log of redaction rectangles.

**Prerequisites:**
- Content-stream round-trip engine (#261) must be solid.
- Secure deletion workflow with explicit confirmation.

**Implementation checklist:**
- [ ] Redaction rectangle model.
- [ ] Apply redaction by rewriting affected content streams.
- [ ] Verify no text remains selectable under redactions.
- [ ] Audit log table.
