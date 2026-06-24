# Frappe — Pre-release Security Audit Checklist

This document is the canonical pre-release security review for the Frappe
Tauri backend. It is built from a manual review of every `#[tauri::command]`
function in `src-tauri/src/commands.rs` and `src-tauri/src/commands_extra.rs`,
plus the underlying `Database` access layer in `src-tauri/src/db.rs`.

The intent is to be re-checked on every release branch. Items marked
**PASS** are confirmed in the current `main`; items marked **TODO** are
known gaps being tracked in the issue tracker.

## 1. Tauri command surface inventory

Every `#[tauri::command]` is registered through `tauri::generate_handler!`
in `src-tauri/src/lib.rs`. As of this writing the full surface is:

| Command | Inputs that touch disk / network | Path validation | URL validation | Notes |
| --- | --- | --- | --- | --- |
| `create_workbook` | `name: String` only | n/a | n/a | in-DB row |
| `list_workbooks` | none | n/a | n/a | in-DB read |
| `delete_workbook` | `id: i64` only | n/a | n/a | in-DB |
| `get_workbook` | `id: i64` only | n/a | n/a | in-DB |
| `create_sheet` / `add_column` / `update_cell_value` / `add_row` / `update_workbook_name` | `name/value: String` | n/a | n/a | in-DB |
| `import_csv_file` / `import_excel_file` | `file_path: String` | `validate_read_path` | n/a | **PASS** |
| `import_google_sheet` / `import_notion_database` | `spreadsheet_id` / `api_key` / `database_id` | n/a | trusts user-supplied HTTPS URL | relies on the upstream API key |
| `preview_import` | `path: String` | `validate_read_path` | n/a | **PASS** |
| `verify_database` | none | n/a | n/a | integrity check |
| `get_business_info` / `save_business_info` | `business_name`, `industry`, … | n/a | n/a | input validated for prefix length and charset |
| `next_order_number` | none | n/a | n/a | |
| `create_invoice` / `list_invoices` / `list_invoices_paginated` / `get_invoice` | `invoice_number` | n/a | n/a | uniqueness via DB constraint |
| `add_invoice_line_item` / `replace_invoice_line_items` / `update_invoice` | numeric + text | n/a | n/a | |
| `create_order` / `list_orders` / `list_orders_paginated` / `get_order` / `update_order_status` / `update_order` | text + enum | n/a | n/a | `update_order_status` enforces the kanban state machine via `db.rs:is_valid_order_transition` |
| `create_estimate` / `list_estimates` / `get_estimate` / `add_estimate_line_item` / `replace_estimate_line_items` / `update_estimate` | text + numeric | n/a | n/a | |
| `add_inventory_item` / `list_inventory_items` / `get_inventory_item` / `adjust_inventory` / `get_low_stock_alerts` / `acknowledge_alert` | numeric + text | n/a | n/a | |
| `create_client` / `list_clients` / `list_clients_paginated` / `get_client` / `update_client` / `delete_client` | text | n/a | n/a | |
| `create_art_approval` / `get_art_approvals_for_order` / `respond_to_art_approval` / `increment_art_approval_follow_up` | text + numeric | n/a | n/a | |
| `record_payment` / `list_payments` / `delete_payment` | numeric | n/a | n/a | |
| `search_invoices_and_orders` | `query: String` | n/a | n/a | SQL uses parameter binding |
| `log_invoice_reminder` / `list_invoice_reminders` | `notes: String` | n/a | n/a | |
| `update_invoice_qb_status` | `status: String` | n/a | n/a | |
| `update_order_job_specs` / `update_order_fulfillment` | text | n/a | n/a | |
| `add_department_note` / `list_department_notes` / `delete_department_note` | `note: String` | n/a | n/a | |
| `open_pdf` | `path: String` | `validate_read_path` | n/a | **PASS** |
| `save_pdf_job` / `list_pdf_jobs` / `delete_pdf_job` | in-DB only | n/a | n/a | cached read (`pdf_jobs_cache`) |
| `create_certified_version` | `file_path: String` | `validate_read_path` | n/a | **PASS** |
| `list_certified_versions` | `id: i64` | n/a | n/a | |
| `render_page_thumbnail` / `render_page` | `path: String` | `validate_read_path` | n/a | **PASS** (Issue #293 added `render_page_b64` that uses the same path validation) |
| `check_fonts` / `check_page_boxes` / `check_image_resolution` / `check_bleed` | `path: String` | `validate_read_path` | n/a | **PASS** |
| `add_bleed` | `path: String`, `output_path: String` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `check_output_intents` / `check_security` / `check_full_preflight` / `check_pdfx` / `check_color_spaces` / `check_overprint` / `check_transparency` / `check_hidden_content` / `check_spot_colors` / `check_ink_coverage` | `path: String` | `validate_read_path` (where applicable) | n/a | **PASS** |
| `list_icc_profiles` | none | n/a | n/a | |
| `convert_rgb_to_cmyk` | `path: String`, `output_path: String` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `add_output_intent` | `path: String`, `output_path: String`, `icc_profile: String` | `validate_read_path` x3 (including ICC profile) | n/a | **PASS** |
| `get_pdf_catalog` | `path: String` | `validate_read_path` | n/a | **PASS** |
| `render_page_with_overprint` / `get_page_dimensions` / `extract_pages` / `delete_pages` / `rotate_page` | `path: String`, optional `output_path` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `reorder_pages` / `insert_blank_page` / `list_layers` / `set_layer_visibility` | `path: String`, optional `output_path` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `decode_content_stream` / `encode_content_stream` / `round_trip_page` / `tokenize_content_stream` | `path: String`, optional `output_path` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `search_text` / `replace_text` | `path: String`, optional `output_path` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `replace_image` / `optimize_image` | `path: String`, `new_image_path: String`, `output_path: String` | `validate_read_path` x2 + `validate_write_path` | n/a | **PASS** |
| `generate_approval_sheet` / `export_preflight_report_json` / `export_preflight_report_csv` / `run_profile` | `path: String` or `run_id: i64` | `validate_read_path` (where applicable) | n/a | **PASS** |
| `create_preflight_profile` / `list_preflight_profiles` / `get_preflight_profile` / `delete_preflight_profile` / `list_profile_checks` / `update_profile_check` / `list_profile_fixups` / `update_profile_fixup` | numeric + text | n/a | n/a | |
| `create_action_list` / `list_action_lists` / `get_action_list` / `delete_action_list` / `add_action_list_step` / `list_action_list_steps` / `delete_action_list_step` / `reorder_action_list_steps` | numeric + text | n/a | n/a | |
| `create_batch_job` / `list_batch_jobs` / `get_batch_job` / `run_batch` / `list_batch_results` | numeric + file paths | `validate_read_path` (in batch runner) | n/a | **PASS** |
| `create_hot_folder` / `list_hot_folders` / `delete_hot_folder` / `toggle_hot_folder` | text | n/a | n/a | cached read (`hot_folders_cache`) |
| `start_hot_folder_watcher` / `stop_hot_folder_watcher` | text | n/a | n/a | writes only to in-process state |
| `compress_pdf` | `path: String`, `output_path: Option<String>` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `redact_pdf` | `path: String`, `output_path: String` | `validate_read_path` + `validate_write_path` | n/a | **PASS** (audit log hash-chain) |
| `get_redaction_audit_log` / `verify_redaction_chain` | `path: String` | n/a | n/a | path is treated as a key only |
| `detect_barcodes` | `path: String` | `validate_read_path` | n/a | **PASS** |
| `get_analytics_summary` / `get_analytics_dashboard` | none | n/a | n/a | |
| `ai_visual_check` | `path: String` (unused stub) | n/a | n/a | returns unimplemented error |
| `save_email_settings` / `get_email_settings` / `send_email` | `attachment_path: Option<String>` | implicit (read in email helper) | uses `lettre` SMTP over rustls | **PASS** |
| `save_ftp_settings` / `get_ftp_settings` / `ftp_upload` | `local_path: String`, `remote_path: String` | validated inside `ftp_upload` | n/a | **PASS** |
| `create_webhook` / `list_webhooks` / `delete_webhook` | `url: String` | n/a | `validate_command_url` | **PASS** (HTTPS-only, no private IPs) |
| `generate_job_ticket` | `output_path: String` | `validate_write_path` | n/a | **PASS** |
| `upload_event_batch_cmd` / `upload_snapshot_cmd` / `get_cloud_backup_status` | `file_path: String` | n/a (snapshot uses path as key) | relies on the cloud backend (stub) | |
| `keychain_read` / `keychain_write` / `keychain_delete` | `service: String`, `key: String` | n/a | n/a | stored in OS keychain (Windows DPAPI / macOS Keychain / Linux Secret Service) |
| `get_schema_version` / `create_backup` / `list_backups` / `export_plaintext_backup` | `output_path: Option<String>` | `validate_write_path` (where applicable) | n/a | **PASS** |
| `reveal_logs` | none | n/a | n/a | opens the OS file manager at the log dir |
| `get_metrics_snapshot` / `crash_report` | `error_message: String` | n/a | trusts the (opt-in) Sentry endpoint via `observability.rs` | |
| `get_preference` / `set_preference` / `get_all_preferences` | text | n/a | n/a | |
| `get_alt_text` / `list_alt_text` / `set_alt_text` | `file_path: String` | n/a (DB key only) | n/a | |
| `set_layer_visibility` | `path: String`, `output_path: String` | `validate_read_path` + `validate_write_path` | n/a | **PASS** |
| `get_debug_session` / `list_debug_sessions` / `create_debug_session` / `delete_debug_session` / `step_forward_debug` / `run_from_here_debug` / `render_debug_thumbnail` / `export_debug_report_pdf` | various | n/a | n/a | |
| `batch_commands` | read-only whitelist | n/a | n/a | rejects any command not in the whitelist |
| `subscribe_events` | `Channel<AppEvent>` | n/a | n/a | server-streamed events |
| `render_page_b64` | `path: String` | `validate_read_path` | n/a | **PASS** |

## 2. Path validation

`validate_read_path` and `validate_write_path` are defined in
`src-tauri/src/commands.rs` and used by every command that takes a
file path argument. `validate_read_path` rejects NUL bytes, requires the
file to exist, and canonicalizes the path. `validate_write_path` also
rejects NUL bytes, requires the parent directory to exist, refuses
parent-traversal (`..`), and blocks system locations
(`C:\Windows`, `C:\Program Files`, `/etc`, `/usr`, `/bin`, …).

**Status: PASS** for every command that takes a path. The audit in
issue #296 closed the only known gap (`check_ink_coverage` was
missing `validate_read_path`; `add_output_intent` was missing it for
the `icc_profile` argument — both now fixed).

## 3. URL validation

`validate_command_url` (in `commands.rs`) is the single source of
truth for URL validation. It enforces:

1. URL is non-empty and ≤ 2048 chars.
2. Scheme is `https` (or `http` only for `localhost` / `*.localhost` /
   `127.0.0.1` / `::1`).
3. Host parses; on DNS failure the request is rejected.
4. **Every resolved IP is checked against `is_blocked_ip`** which
   blocks:
   - `127.0.0.0/8` and `::1` (loopback)
   - `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`
     (private / unique-local)
   - `169.254.0.0/16` and `fe80::/10` (link-local)
   - `0.0.0.0` / `::` (unspecified)
   - Multicast ranges
   - `100.64.0.0/10` (carrier-grade NAT, including the cloud
     metadata service at `169.254.169.254`)

`validate_command_url` is called by `create_webhook` (and by
extension `validate_webhook_url`). HTTP and SMTP transports inside
`lettre` and `suppaftp` use rustls for TLS — no native OpenSSL, no
`native-tls` footgun.

**Status: PASS.**

## 4. SQL injection

Every `rusqlite` call uses parameter binding (`params![]` / `params!`).
There are no string-concatenated SQL statements. The connection runs
in WAL mode and has `PRAGMA foreign_keys = ON` set at startup.

The DB layer is wrapped in a `Mutex<Connection>` (single writer, multiple
readers via short-lived locks). The `QueryCache` (`src-tauri/src/cache.rs`)
sits *outside* the lock and only stores serializable plain data, so
it never holds a reference to the connection across an `.await`.

**Status: PASS.**

## 5. Keychain usage

Secrets are stored in the OS keychain via the `keyring` crate
(`keyring = "3"`). The crate resolves to:

- Windows: Credential Manager (DPAPI-protected)
- macOS: Keychain (kSecClassGenericPassword)
- Linux: Secret Service (D-Bus; requires `libsecret-1-0` at build
  time and `gnome-keyring` / `kwallet` at runtime)

The `DatabaseKey` (used by the `sqlcipher` feature for at-rest
encryption) is the only secret that is ever round-tripped through the
keychain on every `Database::new` call. The `keychain_read`,
`keychain_write`, and `keychain_delete` Tauri commands are exposed for
the frontend to store SMTP / FTP passwords. They use service names
`frappe-email` and `frappe-ftp` to namespace the entries.

**Status: PASS.**

## 6. Content Security Policy

The CSP is defined in `src-tauri/tauri.conf.json`:

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' asset: http://asset.localhost data: blob:;
font-src 'self' data:;
connect-src 'self' ipc: http://ipc.localhost;
object-src 'none';
base-uri 'self';
frame-ancestors 'none';
form-action 'none';
upgrade-insecure-requests;
```

`script-src 'self'` forbids remote script and inline script. No
remote `connect-src` is allowed (the dashboard talks to the Rust
side exclusively over the IPC bridge). The render_page_b64 data
URL path is whitelisted under `img-src`.

**Status: PASS.**

## 7. Dependency audit (`cargo audit`)

Run `cargo audit` from `src-tauri/`. The full output is regenerated
in CI on every push. As of this writing there are no unpatched
vulnerabilities in the dependency tree; the only warning is a
`rustls-webpki` note (informational, not exploitable from the
Tauri-side code path).

For the npm side, `npm audit` is also part of CI. The only allowed
exception is the `react-data-grid` peer dep warning (upstream issue,
not exploitable).

**Status: PASS.**

## 8. IPC surface minimization

`batch_commands` is the only command that accepts an arbitrary name +
args payload. The dispatcher in `commands.rs` rejects any name not
in the read-only whitelist:

```
list_orders, list_invoices, list_clients, get_low_stock_alerts,
get_business_info, list_preflight_profiles, list_hot_folders,
list_pdf_jobs, get_analytics_summary
```

plus the `*_paginated` variants. Every mutating command is not
batched; the frontend must invoke it directly through `invoke()`.

**Status: PASS.**

## 9. Logging

Logs go to `<app_data>/logs/frappe.log` (rotated daily, 14-day
retention). The log format is the standard `tracing-subscriber`
JSON formatter. Secrets are redacted at the field-name level: any
log field named `password`, `secret`, `token`, or `dsn` is replaced
with `***` by the `tracing-subscriber` filter installed in
`src-tauri/src/logging.rs`.

**Status: PASS.**

## 10. Re-audit cadence

- Every PR that touches `commands.rs`, `commands_extra.rs`, or
  `tauri.conf.json` must re-run the checklist before merge.
- A full audit (this entire document) is run on every minor release.
- The `cargo audit` and `npm audit` outputs are attached to the
  release tag as a separate artifact.
