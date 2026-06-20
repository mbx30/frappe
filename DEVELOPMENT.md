# Frappe Development Guide

This guide explains the codebase architecture and patterns used to implement features end-to-end.

## Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| **Frontend** | React 19, TypeScript, Vite | Desktop web UI |
| **IPC** | Tauri 2 | Desktop app framework; bridges React ↔ Rust |
| **Backend** | Rust, rusqlite | Command handlers, business logic, database |
| **Database** | SQLite with WAL mode | Bundled in the app; schema in `src-tauri/src/db.rs` |
| **Design** | Custom system in `src/design-system/` | Reusable button, input, card, etc. components |

## Project Structure

```
frappe/
├── src/                          # React frontend (Vite)
│   ├── components/               # UI components
│   │   ├── ManagementView.tsx    # Main section router
│   │   ├── InvoiceEditor.tsx     # Detailed example of a complex form
│   │   ├── POSView.tsx           # Example with search + dynamic state
│   │   └── ...
│   ├── types.ts                  # TypeScript interfaces mirroring Rust models
│   ├── design-system/            # Reusable UI components (Button, Input, Select, Card, etc.)
│   └── App.tsx                   # Entry point
│
├── src-tauri/src/                # Rust backend
│   ├── lib.rs                    # Tauri setup; registers all commands
│   ├── commands.rs               # Tauri command handlers (invokable from React)
│   ├── db.rs                     # Database schema, migrations, queries
│   ├── models.rs                 # Rust structs (serializable to JSON)
│   ├── pdf/mod.rs                # PDF module (placeholder for Phase 2+)
│   └── cloud_import.rs, import.rs, etc.
│
├── src-tauri/Cargo.toml          # Rust dependencies
├── package.json                  # Node dependencies (React, Tauri CLI, etc.)
├── CLAUDE.md                     # Stack summary + conventions (concise)
└── DEVELOPMENT.md                # This file
```

## Core Patterns

### 1. Adding a Database Table

**Location:** `src-tauri/src/db.rs` → `initialize_schema()` function

```rust
// In the conn.execute_batch() call, add:
CREATE TABLE IF NOT EXISTS my_table (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    value REAL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
```

**For schema updates on existing installs:**
```rust
// Add migrations after table creates in initialize_schema():
let _ = conn.execute("ALTER TABLE my_table ADD COLUMN new_field TEXT DEFAULT ''", []);
```
The `let _ =` silently ignores "column already exists" errors on subsequent app launches.

### 2. Adding a Rust Model

**Location:** `src-tauri/src/models.rs`

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MyRecord {
    pub id: i64,
    pub name: String,
    pub value: f64,
    pub created_at: String,
}
```

**Key rules:**
- Always `#[derive(Debug, Serialize, Deserialize, Clone)]`
- Use `String`, not `&str` (needs to own the data for JSON serialization)
- Use `Option<T>` for nullable fields: `client_id: Option<i64>`
- Dates are `String` in `YYYY-MM-DD` or `YYYY-MM-DD HH:MM:SS` format
- Boolean fields: use `bool`, not `INTEGER` (rusqlite auto-converts)

### 3. Adding a Database Query + Command

**Location:** `src-tauri/src/db.rs` (query logic)

```rust
pub fn get_my_record(&self, id: i64) -> Result<MyRecord> {
    let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
    conn.query_row(
        "SELECT id, name, value, created_at FROM my_table WHERE id = ?1",
        params![id],
        |row| {
            Ok(MyRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                value: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    )
}

pub fn list_my_records(&self) -> Result<Vec<MyRecord>> {
    let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut stmt = conn.prepare("SELECT id, name, value, created_at FROM my_table ORDER BY created_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(MyRecord { id: row.get(0)?, ... })
    })?.collect::<Result<Vec<_>>>();
    rows
}
```

**Location:** `src-tauri/src/commands.rs` (expose to frontend)

```rust
#[tauri::command]
pub fn get_my_record(db: State<'_, Database>, id: i64) -> Result<MyRecord, String> {
    db.get_my_record(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_my_records(db: State<'_, Database>) -> Result<Vec<MyRecord>, String> {
    db.list_my_records().map_err(|e| e.to_string())
}
```

**Location:** `src-tauri/src/lib.rs` (register)

Add to the `invoke_handler!` macro:
```rust
commands::get_my_record,
commands::list_my_records,
```

### 4. Adding TypeScript Types

**Location:** `src/types.ts`

```typescript
export interface MyRecord {
  id: number
  name: string
  value: number
  created_at: string
}
```

**Match the Rust struct exactly.** Tauri serializes Rust types to JSON automatically.

### 5. Building a React Component

**Location:** `src/components/MyComponent.tsx`

Use this template (see `src/components/PaymentPanel.tsx` for a real example):

```typescript
import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Card } from '../design-system'
import type { MyRecord } from '../types'
import './MyComponent.css'

interface MyComponentProps {
  recordId: number
}

export default function MyComponent({ recordId }: MyComponentProps) {
  const [record, setRecord] = useState<MyRecord | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const data = await invoke<MyRecord>('get_my_record', { id: recordId })
      setRecord(data)
    } catch (e) {
      console.error('Failed to load:', e)
    } finally {
      setIsLoading(false)
    }
  }, [recordId])

  useEffect(() => { load() }, [load])

  const handleSave = async () => {
    if (isSaving || !record) return
    setError(null)
    setIsSaving(true)
    try {
      await invoke('update_my_record', { id: record.id, name: record.name })
      await load()
    } catch (e) {
      setError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  if (isLoading || !record) return <div>Loading...</div>

  return (
    <Card>
      <h3>My Component</h3>
      {error && <div className="error">{error}</div>}
      <div className="form-group">
        <label>Name</label>
        <Input
          value={record.name}
          onChange={(e) => setRecord({ ...record, name: e.target.value })}
        />
      </div>
      <Button variant="primary" onClick={handleSave} disabled={isSaving}>
        {isSaving ? 'Saving...' : 'Save'}
      </Button>
    </Card>
  )
}
```

### 6. Common UI Patterns

**isSaving guard + inline error:**
```typescript
const handleSave = async () => {
  if (isSaving) return  // Prevent double-click
  setError(null)
  setIsSaving(true)
  try {
    await invoke(...)
  } catch (e) {
    setError(`Operation failed: ${e}`)  // Show inline, not alert()
  } finally {
    setIsSaving(false)
  }
}
```

**isDirty tracking (show Save button only if changed):**
```typescript
const [isDirty, setIsDirty] = useState(false)

const handleChange = (field: string, value: any) => {
  setForm({ ...form, [field]: value })
  setIsDirty(true)
}

const handleSave = async () => {
  // ... save ...
  setIsDirty(false)  // Clear after successful save
}

// In render:
{isDirty && <Button onClick={handleSave}>Save</Button>}
```

**Validation before save:**
```typescript
const validate = (): string | null => {
  if (!form.name.trim()) return 'Name is required'
  if (form.quantity < 0) return 'Quantity cannot be negative'
  return null
}

const handleSave = async () => {
  const err = validate()
  if (err) { setError(err); return }
  // ... proceed with save
}
```

### 7. Form Patterns

**Line items (invoices, estimates):**
See `src/components/InvoiceEditor.tsx` for the canonical example:
- Client-side state: `lineItems: InvoiceLineItem[]`
- Add/update/remove via array operations + UI binding
- On save for **new** record: call `add_invoice_line_item` for each item
- On save for **existing** record: call `replace_invoice_line_items` with full list (deletes old, inserts new)

**Dropdown with options:**
```typescript
<Select
  value={form.status}
  onChange={(e) => setForm({ ...form, status: e.target.value })}
  options={[
    { value: 'draft', label: 'Draft' },
    { value: 'sent', label: 'Sent' },
  ]}
/>
```

## End-to-End Example: Adding a Simple Feature

**Goal:** Add a "notes" field to invoices.

### Step 1: Database
In `src-tauri/src/db.rs`, add migration:
```rust
let _ = conn.execute("ALTER TABLE invoices ADD COLUMN notes TEXT DEFAULT ''", []);
```

### Step 2: Rust Model
In `src-tauri/src/models.rs`, update `Invoice`:
```rust
pub struct Invoice {
    // ... existing fields ...
    pub notes: String,
}
```

Update the query in `db.rs` to include the new column:
```rust
conn.query_row(
    "SELECT id, ..., notes FROM invoices WHERE id = ?1",
    // ... map new field ...
)
```

### Step 3: Update Command
In `db.rs`, update `update_invoice()` to accept `notes`:
```rust
pub fn update_invoice(&self, id: i64, ..., notes: &str) -> Result<()> {
    conn.execute(
        "UPDATE invoices SET ..., notes = ?N, updated_at = datetime('now') WHERE id = ?1",
        params![..., notes, id],
    )?;
    Ok(())
}
```

In `commands.rs`, update the handler:
```rust
#[tauri::command]
pub fn update_invoice(db: State<'_, Database>, id: i64, ..., notes: String) -> Result<(), String> {
    db.update_invoice(id, ..., &notes).map_err(|e| e.to_string())
}
```

### Step 4: TypeScript
In `src/types.ts`, update `Invoice`:
```typescript
export interface Invoice {
  // ... existing fields ...
  notes: string
}
```

### Step 5: React Component
In `src/components/InvoiceEditor.tsx`:
```typescript
<div className="form-group">
  <label>Notes</label>
  <textarea
    value={invoice.notes}
    onChange={(e) => setInvoice({ ...invoice, notes: e.target.value })}
    placeholder="Internal notes"
    rows={3}
    maxLength={1000}
  />
</div>

// In handleSave:
await invoke('update_invoice', {
  id: invoice.id,
  // ... other fields ...
  notes: invoice.notes.trim(),
})
```

### Step 6: Test
- Open an invoice, add notes, save → verify in DB
- Reload the page → verify notes persist
- Create a new invoice with notes → verify saved correctly

### Step 7: Commit
```bash
git add src-tauri/src/db.rs src-tauri/src/models.rs src-tauri/src/commands.rs src/types.ts src/components/InvoiceEditor.tsx
git commit -m "Add notes field to invoices"
```

## Database Conventions

- **Dates:** Always `YYYY-MM-DD` for dates, `YYYY-MM-DD HH:MM:SS` for timestamps
- **Comparing dates:** Compare as strings, not `new Date()` (they're designed to sort lexicographically)
- **Foreign keys:** Use `Option<i64>` in Rust; nullable in SQL
- **Defaults:** Use `DEFAULT` in schema; don't set in Rust (the DB handles it)
- **Timestamps:** Always add `created_at` and `updated_at` with defaults
- **Nullable JSON:** Use `serde_json::Value` for polymorphic data (e.g., Payment.reference can be check#, card last 4, etc.)

## Frontend Conventions

- **Component files:** Always create a `.tsx` file and a `.css` file in the same directory
- **State naming:** `is<Action>` for booleans (isSaving, isLoading), `<domain><Plural>` for lists (payments, invoices)
- **Error messages:** Show inline in the component, never `alert()`
- **Loading states:** Show loading text or skeleton, not nothing
- **Disabled buttons:** Disable during save; use `isSaving` as the guard
- **Validation:** Call synchronously before save; return `string | null`
- **Dates:** Parse from ISO strings; display with `.split(' ')[0]` for date-only display

## Testing

**Manual testing checklist for any feature:**
1. Create new record → save → verify in list view
2. Edit existing record → change field → save → verify persisted
3. Delete record → verify removed from list
4. Reload page → verify data still there
5. Test edge cases: empty strings, zero values, very long text, special characters

**No automated tests yet.** Use the app manually to verify.

## Git Workflow

1. Work in a worktree: changes are isolated
2. Commit with a clear message: what changed, why
3. Create a PR: title (under 70 chars) + description with test plan
4. Merge to main when ready

## Common Gotchas

| Gotcha | Solution |
|--------|----------|
| Rust field changed but TypeScript not updated | Always update both `models.rs` and `types.ts` together |
| Frontend calls command that doesn't exist | Register it in `lib.rs` after adding to `commands.rs` |
| Dates formatted as timestamps → sort wrong | Store as `YYYY-MM-DD` strings; they sort lexicographically |
| `await invoke()` hangs on error | Wrap in try/catch; don't assume it returns |
| Component re-renders infinitely | Check `useEffect` deps; useCallback + [] deps are your friends |
| Save works but UI doesn't update | Call `load()` after invoke, or manage state directly |
| Line items not saved on edit | Call `replace_*_line_items` before `update_*` (see InvoiceEditor.tsx) |

## Checking Your Work

Before committing:
```bash
cargo check                    # Rust compiles
npx tsc --noEmit             # TypeScript checks pass
```

Before pushing:
```bash
git status                    # Only intended files staged
git diff --staged             # Review your changes
```

## Quick Reference: File Paths

| What | Where |
|------|-------|
| Database schema | `src-tauri/src/db.rs` |
| Rust models | `src-tauri/src/models.rs` |
| Tauri commands | `src-tauri/src/commands.rs` |
| Command registration | `src-tauri/src/lib.rs` |
| React types | `src/types.ts` |
| React components | `src/components/*.tsx` |
| Component styles | `src/components/*.css` |
| Design system | `src/design-system/index.ts` |
| Dependencies (Rust) | `src-tauri/Cargo.toml` |
| Dependencies (Node) | `package.json` |

---

## For AI Assistant-Driven Development

When implementing a feature:
1. Always read the database schema first to understand existing structure
2. Always check existing components for patterns (e.g., PaymentPanel.tsx for payment UX)
3. When adding a command: update Rust model → DB query → Tauri command → TypeScript type → React component
4. Always verify `cargo check` + `tsc --noEmit` before committing
5. Reference this guide; don't assume API details from memory
