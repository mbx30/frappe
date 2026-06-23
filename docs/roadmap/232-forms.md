# Issue #232 — PDF Form Creation and Management

**Goal:** Create interactive PDF forms with fields, buttons, and data export.

**Design sketch:**
- Add AcroForm fields: text, checkbox, radio, dropdown, button.
- Validate field values.
- Export form data to CSV/JSON.

**Prerequisites:**
- lopdf form object manipulation.
- UI for placing fields on a page thumbnail.

**Implementation checklist:**
- [ ] Form field model and storage.
- [ ] Field placement UI.
- [ ] Generate AcroForm dictionary.
- [ ] Export filled form data.
