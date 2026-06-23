# Issue #236 — Accessibility Validation and High Contrast Checking

**Goal:** WCAG compliance checks for PDFs.

**Design sketch:**
- Check contrast ratios.
- Validate color is not sole means of conveying information.
- Check for missing alt-text and structure tags.
- Generate accessibility report.

**Prerequisites:**
- Alt-text (#234) and structure tags (#233) features.
- Color space analysis from preflight checks.

**Implementation checklist:**
- [ ] Contrast checker.
- [ ] Alt-text and structure validators.
- [ ] Accessibility report UI.
