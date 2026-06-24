# Security Module Guide — Issue #296

Quick reference for using the security validators in Tauri commands.

## Overview

The `src-tauri/src/security.rs` module provides defense-in-depth input validation for all Tauri IPC boundaries. Use these functions to validate user-supplied input before any database operations.

## Common Patterns

### Pattern 1: Validate a PDF File Path (Read)

```rust
use crate::security;

#[tauri::command]
pub fn check_pdf_fonts(pdf_path: String) -> Result<FontCheckResult, String> {
    // Validate the read path (null-byte check, existence check, canonicalization)
    let canonical_path = security::validate_read_path(&pdf_path)?;

    // Now use canonical_path in your logic
    let engine = PdfEngine::default();
    engine.check_fonts(&canonical_path)
}
```

### Pattern 2: Validate a PDF Output Path (Write)

```rust
use crate::security;

#[tauri::command]
pub fn compress_pdf(
    db: State<'_, Database>,
    pdf_path: String,
    output_path: String,
) -> Result<(), String> {
    // Validate both input and output paths
    let input = security::validate_read_path(&pdf_path)?;
    let output = security::validate_write_path(&output_path)?;

    // Database operation
    let job = db.create_pdf_job(&input, &output)
        .map_err(|e| e.to_string())?;

    // Perform the compression
    compress(&input, &output)
}
```

### Pattern 3: Validate File Extension Whitelist

```rust
use crate::security;

#[tauri::command]
pub fn import_csv_file(
    db: State<'_, Database>,
    file_path: String,
) -> Result<SheetData, String> {
    // Validate path and restrict to .csv files only
    let canonical = security::validate_read_path_with_extension(&file_path, &["csv"])?;

    // Now import from canonical path
    let (headers, rows) = parse_csv(&canonical)?;
    // ...
}
```

### Pattern 4: Validate User-Supplied Integer (ID)

```rust
use crate::security;

#[tauri::command]
pub fn get_invoice(
    db: State<'_, Database>,
    id: i64,
) -> Result<Invoice, String> {
    // Ensure ID is positive (prevents confusion with error codes)
    security::validate_int_range("invoice_id", id, 1, i64::MAX)?;

    // Perform lookup (database layer will also check existence)
    db.get_invoice(id).map_err(|e| e.to_string())
}
```

### Pattern 5: Validate User-Supplied String

```rust
use crate::security;

#[tauri::command]
pub fn create_workbook(
    db: State<'_, Database>,
    name: String,
) -> Result<Workbook, String> {
    // Ensure name is not empty and not too long
    security::validate_string("workbook_name", &name, 255)?;

    // Create the workbook
    db.create_workbook(&name).map_err(|e| e.to_string())
}
```

### Pattern 6: Alphanumeric Validation

```rust
use crate::security;

#[tauri::command]
pub fn save_business_info(
    db: State<'_, Database>,
    order_prefix: String,
) -> Result<(), String> {
    // Ensure prefix contains only alphanumeric and hyphens
    security::validate_alphanumeric_with_hyphens("order_prefix", &order_prefix)?;
    
    // Validate length separately
    security::validate_string("order_prefix", &order_prefix, 4)?;

    db.save_business_info(&order_prefix).map_err(|e| e.to_string())
}
```

## Error Handling

All security validators return `SecurityResult<T>` which maps to `Result<T, SecurityError>`.

The `SecurityError` type implements `From<SecurityError> for String`, so you can use the `?` operator directly:

```rust
let path = security::validate_read_path(&user_path)?; // Returns String error
```

If you need the error for logging or specific handling:

```rust
match security::validate_read_path(&user_path) {
    Ok(canonical) => { /* use canonical */ },
    Err(e) => {
        // Error is a SecurityError enum
        tracing::error!("Path validation failed: {:?}", e);
        return Err(e.to_string()); // Convert to String for Tauri
    }
}
```

## What Gets Validated?

### Read-Path Validation (`validate_read_path`)

Ensures a path is safe to read from:
- ✅ No null bytes (`\0`)
- ✅ Path exists on filesystem
- ✅ Path is canonicalized (absolute, no `..` components)
- ✅ Returns `PathBuf` for use in subsequent operations

**When to use:** Any command that reads a file supplied by the user.

### Write-Path Validation (`validate_write_path`)

Ensures a path is safe to write to:
- ✅ No null bytes (`\0`)
- ✅ Parent directory exists
- ✅ Not inside system locations (e.g., `/etc`, `C:\Windows`)
- ✅ No parent-directory traversal (`..`)
- ✅ Returns `PathBuf` for use in subsequent operations

**When to use:** Any command that writes a file to a user-specified location.

### Extension Validation (`validate_read_path_with_extension`, `validate_write_path_with_extension`)

Adds file-extension enforcement:
- ✅ All of the above checks
- ✅ File extension must be in the allowed list
- ✅ Case-insensitive matching

**When to use:** CSV import, Excel import, image replacement, PDF operations.

### Integer Validation (`validate_int_range`)

Ensures an integer is within expected bounds:
- ✅ Value is between min and max (inclusive)
- ✅ Rejects negative IDs, out-of-range page indices, etc.

**When to use:** Any numeric parameter (page index, ID, row count, etc.).

### String Validation (`validate_string`)

Ensures a string meets basic requirements:
- ✅ Not empty
- ✅ Length ≤ max_len
- ✅ No null bytes

**When to use:** Names, descriptions, comments, user-supplied text.

### Alphanumeric Validation (`validate_alphanumeric_with_hyphens`)

Ensures a string contains only safe characters:
- ✅ Only `a-z`, `A-Z`, `0-9`, `-`, `_`
- ✅ Prevents injection attacks in prefixes, identifiers

**When to use:** Order prefixes, identifiers, machine-readable strings.

## Testing

Tests for the security module are in `src-tauri/src/security.rs` under `#[cfg(test)]`.

To run security tests:

```bash
cd src-tauri
cargo test security:: --lib
```

To add a regression test for a new validated command:

```rust
#[test]
fn test_my_new_command_rejects_traversal() {
    let result = validate_read_path("../../../etc/passwd");
    assert!(matches!(result, Err(SecurityError::PathTraversalAttempt)));
}
```

## Checklist for Auditing Commands

When auditing a command (see `SECURITY_AUDIT_296.md`), use this checklist:

- [ ] All file-path parameters use `validate_read_path()` or `validate_write_path()`
- [ ] File extensions are whitelisted (if applicable)
- [ ] All ID parameters use `validate_int_range()`
- [ ] All string parameters use `validate_string()` with appropriate max length
- [ ] All identifiers/prefixes use `validate_alphanumeric_with_hyphens()` (if applicable)
- [ ] Error messages don't leak filesystem paths or internal structure
- [ ] Database queries use prepared statements (no string interpolation)
- [ ] No user input is logged without sanitization

---

## Performance Notes

The security validators are designed to be fast:

- `validate_read_path()`: One filesystem `exists()` check + one `canonicalize()` call
- `validate_write_path()`: One `exists()` check on the parent + one `canonicalize()` on the parent
- `validate_int_range()`: Comparison only (O(1))
- `validate_string()`: Length check only (O(1))

All validation happens **before** any database operations, so there's no "wasted" I/O.

---

## References

- [OWASP: Path Traversal](https://owasp.org/www-community/attacks/Path_Traversal)
- [CWE-22: Improper Limitation of a Pathname to a Restricted Directory](https://cwe.mitre.org/data/definitions/22.html)
- [Tauri Security Best Practices](https://tauri.app/docs/building/security)
- Source: `src-tauri/src/security.rs` in this project

