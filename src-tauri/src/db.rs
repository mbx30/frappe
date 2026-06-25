#![allow(unused_mut)] // many functions use `let mut conn` defensively (tx vs non-tx call paths)

use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use fs2::FileExt;

use crate::cache::QueryCache;
use crate::models::*;

/// Backend enforcement of the order status state machine (#160).
///
/// The kanban board (`OrderKanban.tsx`) and order detail view
/// (`OrderDetail.tsx`) both restrict transitions to a strict forward-only
/// flow: `prepress → production → delivery → completed`. Without a matching
/// backend guard, a direct Tauri command invocation could set any arbitrary
/// status string and corrupt an order. This function is the authoritative
/// guard; the frontend maps are a UX convenience layered on top of it.
///
/// A no-op transition (`current == new`) is treated as valid so that an
/// idempotent re-submission of the current status does not error.
pub(crate) fn is_valid_order_transition(current: &str, new: &str) -> bool {
    if current == new {
        return true;
    }
    matches!(
        (current, new),
        ("prepress", "production") | ("production", "delivery") | ("delivery", "completed")
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// A 256-bit SQLCipher encryption key stored in the OS keychain.
#[allow(dead_code)] // Used only when `sqlcipher` feature is enabled
#[derive(Debug, Clone)]
pub struct DatabaseKey {
    raw: [u8; 32],
}

#[allow(dead_code)] // Used only when `sqlcipher` feature is enabled
const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

#[allow(dead_code)] // Used only when `sqlcipher` feature is enabled
fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX_CHARS[(b >> 4) as usize] as char);
        s.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    s
}

#[allow(dead_code)] // Used only when `sqlcipher` feature is enabled
fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            let hi = (s.as_bytes()[i] as char).to_digit(16)?;
            let lo = (s.as_bytes()[i + 1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}

/// Validate a file path is safe to interpolate into SQL. Rejects paths containing
/// SQL special characters (single quotes, backslashes, NUL bytes) that could
/// break out of an ATTACH DATABASE '...' string literal.
#[cfg(feature = "sqlcipher")]
fn validate_sql_path(path: &std::path::Path) -> std::result::Result<&str, String> {
    let s = path
        .to_str()
        .ok_or_else(|| "path contains invalid UTF-8".to_string())?;
    for ch in s.chars() {
        if ch == '\'' || ch == '\0' || ch == '\n' || ch == '\r' {
            return Err(format!("path contains disallowed character: {:?}", ch));
        }
    }
    Ok(s)
}

impl DatabaseKey {
    /// Generate a new random 256-bit key.
    pub fn generate() -> Self {
        let mut raw = [0u8; 32];
        // getrandom should never fail on supported platforms, but if it does
        // (e.g. broken /dev/urandom), panicking is acceptable — this is a
        // fatal startup condition.
        getrandom::getrandom(&mut raw).expect("getrandom failed to generate db key");
        DatabaseKey { raw }
    }

    /// Load key from OS keychain. Returns None if no key is stored.
    #[cfg(feature = "sqlcipher")]
    pub fn load() -> Option<Self> {
        match crate::keychain::read_secret("frappe", "db-key") {
            Ok(secret) if secret.exists => {
                let hex = secret.value?;
                let raw = hex_to_bytes(&hex)?;
                if raw.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&raw);
                    Some(DatabaseKey { raw: arr })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Store key in OS keychain.
    #[cfg(feature = "sqlcipher")]
    pub fn store(&self) -> std::result::Result<(), String> {
        crate::keychain::write_secret("frappe", "db-key", &bytes_to_hex(&self.raw))
    }

    /// Return the key as a hex string for use in PRAGMA key.
    #[cfg(feature = "sqlcipher")]
    pub fn as_hex(&self) -> String {
        bytes_to_hex(&self.raw)
    }
}

pub struct Database {
    pub conn: Mutex<Connection>,
    #[allow(dead_code)] // Used only when `sqlcipher` feature is enabled (and by export commands)
    pub key: DatabaseKey,
    /// Path to the SQLite database file. Stored so `create_backup` (#148) can
    /// open a *separate* connection for `VACUUM INTO` without holding the main
    /// connection's mutex (which would freeze the UI on large DBs).
    db_path: PathBuf,
    /// OS-level file lock held for the lifetime of the process. Prevents a
    /// second app instance from opening the same DB and corrupting it.
    /// Acquired in `Database::new` via `fs2::FileExt::try_lock_exclusive`;
    /// the lock is released when the file handle is dropped at process exit.
    _db_lock: Mutex<Option<std::fs::File>>,
    /// 30 s TTL cache for the PDF job list. Invalidated on save / delete
    /// (#252). The dashboard polls this list every few seconds when on the
    /// PDF view screen; without a cache the read is the hottest path in the
    /// app.
    pdf_jobs_cache: QueryCache<Vec<PdfSummary>>,
    /// 60 s TTL cache for the preflight profile list. Invalidated on any
    /// create / delete / update of a profile (#252).
    preflight_profiles_cache: QueryCache<Vec<PreflightProfile>>,
    /// 30 s TTL cache for the hot-folder list. Invalidated on any
    /// create / delete / toggle of a hot folder (#252).
    hot_folders_cache: QueryCache<Vec<HotFolder>>,
    /// 60 s TTL cache for the singleton business-info row (#252). The
    /// business header is rendered on every PDF so this is on the hot path.
    business_info_cache: QueryCache<Option<BusinessInfo>>,
}

impl Database {
    pub fn new(app_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("frappe.db");

        // Acquire an OS-level exclusive lock on a sidecar `frappe.lock` file
        // (#147). The previous implementation used `OpenOptions::create_new`
        // with an `AlreadyExists` fallback that truncated the file and claimed
        // the lock — but `truncate` succeeds even when a live instance holds
        // the file, so two instances could run simultaneously. `fs2`'s
        // `try_lock_exclusive` asks the OS whether another process already
        // holds the lock, which is the only reliable single-instance guard.
        // The file handle (and thus the lock) is kept alive in the `Database`
        // struct for the lifetime of the process.
        let lock_path = app_dir.join("frappe.lock");
        let lock_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    format!("Cannot create lock file {}: {}", lock_path.display(), e).into(),
                )
            })?;
        lock_file.try_lock_exclusive().map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(
                format!(
                    "Another Frappe instance is already running. Close it and try again. ({})",
                    e
                )
                .into(),
            )
        })?;

        #[cfg(feature = "sqlcipher")]
        let db_exists = db_path.exists();

        #[cfg(feature = "sqlcipher")]
        let key = {
            if db_exists {
                DatabaseKey::load().ok_or_else(|| {
                    let msg = "Database is encrypted but no encryption key found in OS keychain. \
                               This happens after a full OS reinstall or keychain wipe. \
                               Restore your database from a plaintext backup (if you have one) \
                               or contact support for recovery options."
                        .to_string();
                    rusqlite::Error::ToSqlConversionFailure(msg.into())
                })?
            } else {
                let new_key = DatabaseKey::generate();
                new_key.store().map_err(|e| {
                    let msg = format!("failed to store db encryption key: {}", e);
                    rusqlite::Error::ToSqlConversionFailure(msg.into())
                })?;
                new_key
            }
        };

        #[cfg(not(feature = "sqlcipher"))]
        let key = DatabaseKey::generate(); // unused placeholder, keeps struct shape consistent

        #[cfg(feature = "sqlcipher")]
        {
            let key_hex = key.as_hex();

            if db_exists {
                // Check if DB is already encrypted by opening with the key
                let test_conn = Connection::open(&db_path)?;
                test_conn.execute_batch(&format!("PRAGMA key = \"x'{}'\"", key_hex))?;
                let has_schema: bool = test_conn
                    .query_row(
                        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                if !has_schema {
                    // Move the OS lock into the migrated database so it stays
                    // held for the process lifetime (the early return drops
                    // this stack frame, which would otherwise release it).
                    return Self::migrate_from_plaintext(&db_path, &key, lock_file);
                }
            }

            let conn = Connection::open(&db_path)?;
            conn.execute_batch(&format!("PRAGMA key = \"x'{}'\"", key_hex))?;
            conn.execute_batch("PRAGMA cipher_compatibility = 4")?;
            conn.execute_batch("PRAGMA cipher_page_size = 4096")?;
            conn.execute_batch("PRAGMA journal_mode = WAL")?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            let db = Database {
                conn: Mutex::new(conn),
                key,
                db_path: db_path.clone(),
                _db_lock: Mutex::new(Some(lock_file)),
                pdf_jobs_cache: QueryCache::with_ttl(4, Duration::from_secs(30)),
                preflight_profiles_cache: QueryCache::with_ttl(8, Duration::from_secs(60)),
                hot_folders_cache: QueryCache::with_ttl(4, Duration::from_secs(30)),
                business_info_cache: QueryCache::with_ttl(2, Duration::from_secs(60)),
            };
            db.initialize_schema()?;
            return Ok(db);
        }

        #[cfg(not(feature = "sqlcipher"))]
        {
            let _ = key; // silence unused
            let conn = Connection::open(&db_path)?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
            let db = Database {
                conn: Mutex::new(conn),
                key: DatabaseKey::generate(), // placeholder
                db_path: db_path.clone(),
                _db_lock: Mutex::new(Some(lock_file)),
                pdf_jobs_cache: QueryCache::with_ttl(4, Duration::from_secs(30)),
                preflight_profiles_cache: QueryCache::with_ttl(8, Duration::from_secs(60)),
                hot_folders_cache: QueryCache::with_ttl(4, Duration::from_secs(30)),
                business_info_cache: QueryCache::with_ttl(2, Duration::from_secs(60)),
            };
            db.initialize_schema()?;
            Ok(db)
        }
    }

    /// Migrate an existing plaintext SQLite database to encrypted SQLCipher format.
    /// The plaintext backup at `.db.pre-encrypt` is removed on success AND on
    /// any failure during migration (so unencrypted PII doesn't linger on disk
    /// if the migration crashes partway through).
    #[cfg(feature = "sqlcipher")]
    fn migrate_from_plaintext(
        db_path: &PathBuf,
        key: &DatabaseKey,
        lock_file: std::fs::File,
    ) -> Result<Self> {
        let key_hex = key.as_hex();

        // Save old plaintext DB
        let backup_path = db_path.with_extension("db.pre-encrypt");
        std::fs::copy(db_path, &backup_path).map_err(|e| {
            let msg = format!("failed to backup plaintext db: {}", e);
            rusqlite::Error::ToSqlConversionFailure(msg.into())
        })?;

        // Guard: ensure the plaintext backup is always cleaned up, even on
        // panic or early return. `commit()` on success keeps it for the caller
        // to inspect, but our success path doesn't keep it (we delete it).
        struct PlaintextGuard(std::path::PathBuf);
        impl Drop for PlaintextGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _guard = PlaintextGuard(backup_path.clone());

        // Open plaintext DB with SQLCipher compat mode (no encryption)
        let plain_conn = Connection::open(&backup_path)?;
        plain_conn.execute_batch(
            "PRAGMA key = '';
             PRAGMA cipher_use_hmac = OFF;
             PRAGMA kdf_iter = 0;
             PRAGMA cipher_page_size = 1024;
             PRAGMA foreign_keys = OFF;",
        )?;

        // Create new encrypted DB via ATTACH + sqlcipher_export
        let enc_path_str = validate_sql_path(db_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        plain_conn.execute_batch(&format!(
            "ATTACH DATABASE '{}' AS encrypted KEY \"x'{}'\";",
            enc_path_str, key_hex
        ))?;
        plain_conn.execute_batch("SELECT sqlcipher_export('encrypted');")?;
        plain_conn.execute_batch("DETACH DATABASE encrypted;")?;
        drop(plain_conn); // close before the guard removes the backup

        // Open the newly encrypted DB
        let conn = Connection::open(db_path)?;
        conn.execute_batch(&format!("PRAGMA key = \"x'{}'\"", key_hex))?;
        conn.execute_batch("PRAGMA cipher_compatibility = 4")?;
        conn.execute_batch("PRAGMA cipher_page_size = 4096")?;
        conn.execute_batch("PRAGMA journal_mode = WAL")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Mark schema version 2 for migration
        conn.execute("INSERT OR REPLACE INTO schema_version (version, migrated_at) VALUES (2, datetime('now'))", [])?;

        Ok(Database {
            conn: Mutex::new(conn),
            key: key.clone(),
            db_path: db_path.clone(),
            _db_lock: Mutex::new(Some(lock_file)),
            pdf_jobs_cache: QueryCache::with_ttl(4, Duration::from_secs(30)),
            preflight_profiles_cache: QueryCache::with_ttl(8, Duration::from_secs(60)),
            hot_folders_cache: QueryCache::with_ttl(4, Duration::from_secs(30)),
            business_info_cache: QueryCache::with_ttl(2, Duration::from_secs(60)),
        })
    }

    fn initialize_schema(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                migrated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT OR IGNORE INTO schema_version (version) VALUES (1);
            CREATE TABLE IF NOT EXISTS event_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tenant_id TEXT NOT NULL DEFAULT 'default',
                entity_type TEXT NOT NULL,
                entity_id INTEGER NOT NULL,
                action TEXT NOT NULL,
                payload TEXT DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_event_log_tenant ON event_log(tenant_id);
            CREATE INDEX IF NOT EXISTS idx_event_log_entity ON event_log(entity_type, entity_id);
            CREATE TABLE IF NOT EXISTS backup_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                backup_type TEXT NOT NULL,
                file_path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                checksum TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS business_info (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                tenant_id TEXT NOT NULL DEFAULT 'default',
                business_name TEXT,
                industry TEXT,
                company_size TEXT,
                order_number_prefix TEXT DEFAULT '',
                completed_onboarding INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS workbooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tenant_id TEXT NOT NULL DEFAULT 'default',
                name TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS sheets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workbook_id INTEGER NOT NULL REFERENCES workbooks(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS sheet_columns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sheet_id INTEGER NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                col_type TEXT NOT NULL DEFAULT 'text',
                sort_order INTEGER NOT NULL DEFAULT 0,
                width INTEGER DEFAULT 150
            );
            CREATE TABLE IF NOT EXISTS cell_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sheet_id INTEGER NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
                row_index INTEGER NOT NULL,
                column_id INTEGER NOT NULL REFERENCES sheet_columns(id) ON DELETE CASCADE,
                value TEXT DEFAULT '',
                UNIQUE(sheet_id, row_index, column_id)
            );
            CREATE INDEX IF NOT EXISTS idx_cell_data_sheet ON cell_data(sheet_id, row_index);
            CREATE INDEX IF NOT EXISTS idx_cell_data_column ON cell_data(sheet_id, column_id);
            CREATE TABLE IF NOT EXISTS invoices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                invoice_number TEXT NOT NULL UNIQUE,
                client_id INTEGER,
                status TEXT NOT NULL DEFAULT 'draft',
                issue_date TEXT NOT NULL,
                due_date TEXT NOT NULL,
                payment_terms TEXT DEFAULT 'net-30',
                subtotal REAL NOT NULL DEFAULT 0,
                tax_rate REAL NOT NULL DEFAULT 0,
                tax_amount REAL NOT NULL DEFAULT 0,
                total REAL NOT NULL DEFAULT 0,
                currency TEXT DEFAULT 'USD',
                internal_notes TEXT DEFAULT '',
                customer_notes TEXT DEFAULT '',
                qb_sync_status TEXT DEFAULT 'not_synced',
                amount_paid REAL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS invoice_line_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
                description TEXT NOT NULL,
                quantity REAL NOT NULL DEFAULT 1,
                unit_price REAL NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_invoices_number ON invoices(invoice_number);
            CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
            CREATE INDEX IF NOT EXISTS idx_invoice_items ON invoice_line_items(invoice_id);
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_number TEXT NOT NULL UNIQUE,
                client_id INTEGER,
                status TEXT NOT NULL DEFAULT 'prepress',
                priority TEXT DEFAULT 'normal',
                due_date TEXT NOT NULL,
                description TEXT DEFAULT '',
                artwork_notes TEXT DEFAULT '',
                artwork_url TEXT,
                artwork_approved INTEGER DEFAULT 0,
                deposit_requested INTEGER DEFAULT 0,
                deposit_amount REAL DEFAULT 0,
                total_value REAL DEFAULT 0,
                print_type TEXT DEFAULT '',
                paper_stock TEXT DEFAULT '',
                ink_colors TEXT DEFAULT '',
                finishing TEXT DEFAULT '',
                quantity INTEGER DEFAULT 0,
                production_notes TEXT DEFAULT '',
                assigned_operator TEXT DEFAULT '',
                fulfillment_method TEXT DEFAULT 'pickup',
                tracking_number TEXT DEFAULT '',
                tracking_carrier TEXT DEFAULT '',
                ready_for_pickup INTEGER DEFAULT 0,
                shipped_at TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS order_status_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
                previous_status TEXT NOT NULL,
                new_status TEXT NOT NULL,
                notes TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_orders_number ON orders(order_number);
            CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
            CREATE INDEX IF NOT EXISTS idx_orders_due_date ON orders(due_date);
            CREATE INDEX IF NOT EXISTS idx_order_history ON order_status_history(order_id);
            CREATE TABLE IF NOT EXISTS estimates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                estimate_number TEXT NOT NULL UNIQUE,
                client_id INTEGER,
                status TEXT NOT NULL DEFAULT 'draft',
                valid_until TEXT NOT NULL,
                subtotal REAL NOT NULL DEFAULT 0,
                tax_rate REAL NOT NULL DEFAULT 0,
                tax_amount REAL NOT NULL DEFAULT 0,
                total REAL NOT NULL DEFAULT 0,
                currency TEXT DEFAULT 'USD',
                notes TEXT DEFAULT '',
                artwork_requirements TEXT DEFAULT '',
                converted_order_id INTEGER,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS estimate_line_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                estimate_id INTEGER NOT NULL REFERENCES estimates(id) ON DELETE CASCADE,
                description TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'materials',
                quantity REAL NOT NULL DEFAULT 1,
                unit_price REAL NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_estimates_number ON estimates(estimate_number);
            CREATE INDEX IF NOT EXISTS idx_estimates_status ON estimates(status);
            CREATE INDEX IF NOT EXISTS idx_estimates_valid_until ON estimates(valid_until);
            CREATE INDEX IF NOT EXISTS idx_estimate_items ON estimate_line_items(estimate_id);
            CREATE TABLE IF NOT EXISTS inventory_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                material_type TEXT NOT NULL,
                size TEXT NOT NULL,
                attributes TEXT DEFAULT '',
                quantity REAL NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT 'pieces',
                reorder_level REAL DEFAULT 0,
                alert_type TEXT DEFAULT 'quantity',
                alert_threshold REAL DEFAULT 0,
                last_restocked TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS inventory_transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                inventory_item_id INTEGER NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
                transaction_type TEXT NOT NULL,
                quantity_change REAL NOT NULL,
                reason TEXT NOT NULL,
                related_order_id INTEGER,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS inventory_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                inventory_item_id INTEGER NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
                alert_type TEXT NOT NULL,
                current_quantity REAL NOT NULL,
                threshold REAL NOT NULL,
                is_acknowledged INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_inventory_material ON inventory_items(material_type);
            CREATE INDEX IF NOT EXISTS idx_inventory_quantity ON inventory_items(quantity);
            CREATE INDEX IF NOT EXISTS idx_transactions_item ON inventory_transactions(inventory_item_id);
            CREATE INDEX IF NOT EXISTS idx_alerts_item ON inventory_alerts(inventory_item_id);
            CREATE TABLE IF NOT EXISTS clients (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                company TEXT DEFAULT '',
                email TEXT DEFAULT '',
                phone TEXT DEFAULT '',
                address TEXT DEFAULT '',
                tags TEXT DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                notes TEXT DEFAULT '',
                last_contacted TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_clients_name ON clients(name);
            CREATE INDEX IF NOT EXISTS idx_clients_company ON clients(company);
            CREATE INDEX IF NOT EXISTS idx_clients_status ON clients(status);
            CREATE TABLE IF NOT EXISTS art_approvals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
                version INTEGER NOT NULL DEFAULT 1,
                file_path TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'pending',
                customer_notes TEXT DEFAULT '',
                staff_notes TEXT DEFAULT '',
                secure_token TEXT NOT NULL UNIQUE,
                follow_up_hours INTEGER NOT NULL DEFAULT 24,
                follow_up_count INTEGER NOT NULL DEFAULT 0,
                submitted_at TEXT DEFAULT (datetime('now')),
                responded_at TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_approvals_order ON art_approvals(order_id);
            CREATE INDEX IF NOT EXISTS idx_approvals_token ON art_approvals(secure_token);
            CREATE INDEX IF NOT EXISTS idx_approvals_status ON art_approvals(status);
            CREATE TABLE IF NOT EXISTS payments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                invoice_id INTEGER REFERENCES invoices(id) ON DELETE SET NULL,
                order_id INTEGER REFERENCES orders(id) ON DELETE SET NULL,
                amount REAL NOT NULL,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                reference TEXT DEFAULT '',
                notes TEXT DEFAULT '',
                recorded_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_payments_invoice ON payments(invoice_id);
            CREATE INDEX IF NOT EXISTS idx_payments_order ON payments(order_id);
            CREATE TABLE IF NOT EXISTS invoice_reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
                method TEXT NOT NULL DEFAULT 'email',
                notes TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_reminders_invoice ON invoice_reminders(invoice_id);
            CREATE TABLE IF NOT EXISTS department_notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
                note TEXT NOT NULL,
                department TEXT NOT NULL DEFAULT 'general',
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_dept_notes_order ON department_notes(order_id);
            CREATE TABLE IF NOT EXISTS preflight_run_summary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL REFERENCES pdf_jobs(id) ON DELETE CASCADE,
                profile TEXT NOT NULL DEFAULT 'full',
                total_errors INTEGER NOT NULL DEFAULT 0,
                total_warnings INTEGER NOT NULL DEFAULT 0,
                total_ok INTEGER NOT NULL DEFAULT 0,
                ran_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_preflight_run_job ON preflight_run_summary(job_id);
            CREATE TABLE IF NOT EXISTS preflight_findings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL REFERENCES preflight_run_summary(id) ON DELETE CASCADE,
                check_name TEXT NOT NULL,
                severity TEXT NOT NULL,
                page_num INTEGER,
                object_ref TEXT,
                message TEXT NOT NULL,
                fix_hint TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_preflight_findings_run ON preflight_findings(run_id);
            CREATE TABLE IF NOT EXISTS preflight_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                is_builtin INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS profile_checks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                profile_id INTEGER NOT NULL REFERENCES preflight_profiles(id) ON DELETE CASCADE,
                check_name TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'error',
                enabled INTEGER NOT NULL DEFAULT 1,
                params TEXT DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS profile_fixups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                profile_id INTEGER NOT NULL REFERENCES preflight_profiles(id) ON DELETE CASCADE,
                fixup_name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                params TEXT DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS action_lists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS action_list_steps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action_list_id INTEGER NOT NULL REFERENCES action_lists(id) ON DELETE CASCADE,
                step_order INTEGER NOT NULL DEFAULT 0,
                action_type TEXT NOT NULL,
                params TEXT DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS batch_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action_list_id INTEGER NOT NULL REFERENCES action_lists(id) ON DELETE CASCADE,
                status TEXT NOT NULL DEFAULT 'pending',
                total_files INTEGER NOT NULL DEFAULT 0,
                processed_files INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                started_at TEXT,
                completed_at TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS batch_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                batch_id INTEGER NOT NULL REFERENCES batch_jobs(id) ON DELETE CASCADE,
                file_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                output_path TEXT,
                error_message TEXT,
                started_at TEXT,
                completed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS hot_folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                watch_path TEXT NOT NULL,
                action_list_id INTEGER NOT NULL REFERENCES action_lists(id) ON DELETE CASCADE,
                output_path TEXT NOT NULL,
                file_pattern TEXT DEFAULT '*.pdf',
                is_active INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS email_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                smtp_host TEXT NOT NULL DEFAULT '',
                smtp_port INTEGER NOT NULL DEFAULT 587,
                smtp_username TEXT NOT NULL DEFAULT '',
                smtp_password TEXT NOT NULL DEFAULT '',
                use_tls INTEGER DEFAULT 1,
                from_address TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS ftp_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                host TEXT NOT NULL DEFAULT '',
                port INTEGER NOT NULL DEFAULT 21,
                username TEXT NOT NULL DEFAULT '',
                password TEXT NOT NULL DEFAULT '',
                remote_dir TEXT NOT NULL DEFAULT '/'
            );
            CREATE TABLE IF NOT EXISTS webhooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                event TEXT NOT NULL,
                is_active INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS pdf_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                page_count INTEGER NOT NULL,
                pdf_version TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                creator TEXT NOT NULL DEFAULT '',
                producer TEXT NOT NULL DEFAULT '',
                is_encrypted INTEGER NOT NULL DEFAULT 0,
                creation_date TEXT NOT NULL DEFAULT '',
                opened_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS pdf_certified_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL REFERENCES pdf_jobs(id) ON DELETE CASCADE,
                version_number INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL DEFAULT 0,
                author TEXT NOT NULL DEFAULT '',
                comment TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                is_signed INTEGER NOT NULL DEFAULT 0,
                UNIQUE(job_id, version_number)
            );
            -- Redaction audit trail (#231). Each row is one applied redaction
            -- operation. `content_hash` is the SHA-256 of the resulting PDF and
            -- `previous_hash` links to the prior operation for the same source
            -- file, forming a tamper-evident hash-chain. Triggers below enforce
            -- append-only immutability so the trail can serve as legal evidence.
            CREATE TABLE IF NOT EXISTS redaction_operations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_path TEXT NOT NULL,
                output_path TEXT NOT NULL,
                operator_name TEXT NOT NULL DEFAULT '',
                redaction_count INTEGER NOT NULL DEFAULT 0,
                pages_modified INTEGER NOT NULL DEFAULT 0,
                regions_json TEXT NOT NULL DEFAULT '[]',
                content_hash TEXT NOT NULL,
                previous_hash TEXT,
                notes TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_redaction_ops_source
                ON redaction_operations(source_path);
            CREATE TRIGGER IF NOT EXISTS redaction_ops_no_update
                BEFORE UPDATE ON redaction_operations
            BEGIN
                SELECT RAISE(ABORT, 'redaction_operations is append-only');
            END;
            CREATE TRIGGER IF NOT EXISTS redaction_ops_no_delete
                BEFORE DELETE ON redaction_operations
            BEGIN
                SELECT RAISE(ABORT, 'redaction_operations is append-only');
            END;
            -- Key/value preference store (#241, #275). One row per
            -- preference name. Updated in place via INSERT OR REPLACE.
            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            -- Image alt-text overrides (#234). One row per
            -- (file_path, object_id) tuple; updated via INSERT OR
            -- REPLACE. Used by the alt-text renderer and the
            -- AccessibilityCheck sweep.
            CREATE TABLE IF NOT EXISTS image_alt_text (
                file_path TEXT NOT NULL,
                object_id INTEGER NOT NULL,
                alt_text TEXT NOT NULL DEFAULT '',
                is_decorative INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (file_path, object_id)
            );
            -- PDF annotations (#230): highlights, sticky notes, underline, strikethrough.
            -- Non-destructive: stored as metadata, not burned into the PDF bytes.
            CREATE TABLE IF NOT EXISTS pdf_annotations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                page INTEGER NOT NULL,
                annotation_type TEXT NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                width REAL NOT NULL,
                height REAL NOT NULL,
                color TEXT NOT NULL DEFAULT '#FFD700',
                content TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_annotations_file_page ON pdf_annotations(file_path, page);
            CREATE TABLE IF NOT EXISTS pdf_annotation_replies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                annotation_id INTEGER NOT NULL REFERENCES pdf_annotations(id) ON DELETE CASCADE,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );"
        )?;
        // Seed built-in preflight profiles
        let profile_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM preflight_profiles", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);
        if profile_count == 0 {
            let builtins = vec![
                (
                    "PDF/X-1a",
                    "Strict PDF/X-1a compliance check",
                    vec![
                        ("fonts_embedded", "error"),
                        ("page_boxes", "error"),
                        ("image_resolution", "error"),
                        ("bleed", "error"),
                        ("output_intents", "error"),
                        ("security", "error"),
                        ("pdfx_version", "error"),
                        ("color_spaces", "error"),
                    ],
                ),
                (
                    "PDF/X-4",
                    "PDF/X-4 compliance check with transparency support",
                    vec![
                        ("fonts_embedded", "error"),
                        ("page_boxes", "error"),
                        ("image_resolution", "warning"),
                        ("bleed", "error"),
                        ("output_intents", "error"),
                        ("security", "error"),
                        ("pdfx_version", "error"),
                        ("color_spaces", "error"),
                    ],
                ),
                (
                    "Print Ready",
                    "General print-ready check for commercial printing",
                    vec![
                        ("fonts_embedded", "error"),
                        ("page_boxes", "warning"),
                        ("image_resolution", "error"),
                        ("bleed", "error"),
                        ("security", "warning"),
                        ("overprint", "warning"),
                        ("transparency", "warning"),
                        ("spot_colors", "info"),
                    ],
                ),
                (
                    "Web/Mobile",
                    "Lightweight check for digital distribution",
                    vec![
                        ("fonts_embedded", "warning"),
                        ("page_boxes", "info"),
                        ("image_resolution", "warning"),
                        ("security", "error"),
                    ],
                ),
            ];
            for (name, desc, checks) in &builtins {
                if let Err(e) = conn.execute(
                    "INSERT INTO preflight_profiles (name, description, is_builtin) VALUES (?1, ?2, 1)",
                    params![name, desc],
                ) {
                    tracing::warn!("failed to seed preflight profile '{}': {}", name, e);
                    continue;
                }
                let pid = conn.last_insert_rowid();
                for (check_name, severity) in checks {
                    if let Err(e) = conn.execute(
                        "INSERT INTO profile_checks (profile_id, check_name, severity, enabled) VALUES (?1, ?2, ?3, 1)",
                        params![pid, check_name, severity],
                    ) {
                        tracing::warn!("failed to seed check '{}' for profile '{}': {}", check_name, name, e);
                    }
                }
            }
        }

        // Migration: add columns only if they don't already exist
        //
        // #168: `table` and every token in `col_def` are validated against a
        // strict allowlist to prevent DDL injection. The leading token is
        // checked as an identifier; subsequent tokens must match known SQL
        // DDL keywords, identifiers, numeric/single-quoted literals, or
        // balanced parentheses.
        fn is_valid_identifier(s: &str) -> bool {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }

        fn is_valid_col_def(s: &str) -> bool {
            let mut depth = 0i32;
            let mut i = 0;
            let bytes = s.as_bytes();
            while i < bytes.len() {
                let c = bytes[i] as char;
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth < 0 {
                            return false;
                        }
                    }
                    '\'' => {
                        i += 1;
                        while i < bytes.len() && bytes[i] as char != '\'' {
                            if bytes[i] as char == '\\' {
                                i += 1; // skip escaped char
                            }
                            i += 1;
                        }
                        if i >= bytes.len() {
                            return false; // unclosed quote
                        }
                    }
                    ';' | '-' => {
                        // Disallow bare semicolons; '-' is allowed only inside
                        // a string literal or after a '(' (negative number).
                        // Outside those contexts it could start a comment.
                        if c == '-' {
                            // Check if this is a single-line comment start
                            if i + 1 < bytes.len() && bytes[i + 1] as char == '-' {
                                return false;
                            }
                            if depth == 0 {
                                // bare '-' outside a type/check context is suspicious
                                // but allow as part of DEFAULT -1 or similar
                            }
                        } else {
                            return false; // bare ';' is never valid in a col_def
                        }
                    }
                    '/' => {
                        if i + 1 < bytes.len() && bytes[i + 1] as char == '*' {
                            return false; // block comment start
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            depth == 0 // parens must be balanced
        }

        fn add_col_if_missing(
            conn: &Connection,
            table: &str,
            col_def: &str,
        ) -> rusqlite::Result<()> {
            let col_name = col_def.split_whitespace().next().unwrap_or("");
            if !is_valid_identifier(table)
                || !is_valid_identifier(col_name)
                || !is_valid_col_def(col_def)
            {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
                    params![table, col_name],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if !exists {
                conn.execute(&format!("ALTER TABLE {} ADD COLUMN {}", table, col_def), [])?;
            }
            Ok(())
        }

        add_col_if_missing(&conn, "orders", "tenant_id TEXT NOT NULL DEFAULT 'default'")?;
        add_col_if_missing(&conn, "orders", "print_type TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "paper_stock TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "ink_colors TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "finishing TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "quantity INTEGER DEFAULT 0")?;
        add_col_if_missing(&conn, "orders", "production_notes TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "assigned_operator TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "fulfillment_method TEXT DEFAULT 'pickup'")?;
        add_col_if_missing(&conn, "orders", "tracking_number TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "tracking_carrier TEXT DEFAULT ''")?;
        add_col_if_missing(&conn, "orders", "ready_for_pickup INTEGER DEFAULT 0")?;
        add_col_if_missing(&conn, "orders", "shipped_at TEXT")?;
        add_col_if_missing(
            &conn,
            "invoices",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "invoices",
            "qb_sync_status TEXT DEFAULT 'not_synced'",
        )?;
        add_col_if_missing(&conn, "invoices", "amount_paid REAL DEFAULT 0")?;
        add_col_if_missing(
            &conn,
            "estimates",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "clients",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "inventory_items",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "art_approvals",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "payments",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "pdf_jobs",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "preflight_run_summary",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "action_lists",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "batch_jobs",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "hot_folders",
            "tenant_id TEXT NOT NULL DEFAULT 'default'",
        )?;
        add_col_if_missing(
            &conn,
            "business_info",
            "order_number_prefix TEXT DEFAULT ''",
        )?;
        Ok(())
    }

    pub fn create_workbook(&self, name: &str) -> Result<Workbook> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        // Reject duplicate workbook names to avoid data confusion and export
        // collisions (#190). Pre-check rather than relying on a UNIQUE
        // constraint so the error is a clear, user-facing message.
        let existing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM workbooks WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        if existing > 0 {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                Some(format!("A workbook named \"{}\" already exists", name)),
            ));
        }
        conn.execute("INSERT INTO workbooks (name) VALUES (?1)", params![name])?;
        let id = conn.last_insert_rowid();
        Ok(Workbook {
            id,
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_workbooks(&self) -> Result<Vec<Workbook>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, created_at, updated_at FROM workbooks ORDER BY updated_at DESC LIMIT 500",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Workbook {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_workbook(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM workbooks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Redaction audit trail (#231) ────────────────────────────────────

    /// Append a redaction operation to the audit hash-chain and return its new
    /// row id. The new row's `previous_hash` is automatically linked to the
    /// most recent operation's `content_hash` for the same `source_path`,
    /// forming a tamper-evident chain. Rows are append-only (enforced by
    /// triggers), so callers cannot rewrite history.
    pub fn log_redaction_operation(
        &self,
        source_path: &str,
        output_path: &str,
        content_hash: &str,
        regions_json: &str,
        redaction_count: i64,
        pages_modified: i64,
        operator_name: &str,
        notes: &str,
    ) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        // Link to the latest prior operation for this source file.
        let previous_hash: Option<String> = conn
            .query_row(
                "SELECT content_hash FROM redaction_operations
                 WHERE source_path = ?1 ORDER BY id DESC LIMIT 1",
                params![source_path],
                |row| row.get(0),
            )
            .ok();
        conn.execute(
            "INSERT INTO redaction_operations
                (source_path, output_path, operator_name, redaction_count,
                 pages_modified, regions_json, content_hash, previous_hash, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                source_path,
                output_path,
                operator_name,
                redaction_count,
                pages_modified,
                regions_json,
                content_hash,
                previous_hash,
                notes,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Return the redaction audit chain for a source file, oldest first, with
    /// each entry's `chain_valid` flag computed by checking that its
    /// `previous_hash` matches the preceding entry's `content_hash`.
    pub fn query_redaction_log(&self, source_path: &str) -> Result<Vec<RedactionAuditEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, source_path, output_path, operator_name, redaction_count,
                    pages_modified, regions_json, content_hash, previous_hash,
                    notes, created_at
             FROM redaction_operations
             WHERE source_path = ?1
             ORDER BY id ASC
             LIMIT 1000",
        )?;
        let rows = stmt.query_map(params![source_path], |row| {
            Ok(RedactionAuditEntry {
                id: row.get(0)?,
                source_path: row.get(1)?,
                output_path: row.get(2)?,
                operator_name: row.get(3)?,
                redaction_count: row.get(4)?,
                pages_modified: row.get(5)?,
                regions_json: row.get(6)?,
                content_hash: row.get(7)?,
                previous_hash: row.get(8)?,
                notes: row.get(9)?,
                created_at: row.get(10)?,
                chain_valid: true, // recomputed below
            })
        })?;
        let mut entries: Vec<RedactionAuditEntry> = rows.collect::<Result<Vec<_>>>()?;

        // Walk the chain in order, validating each link against its predecessor.
        let mut prev_hash: Option<String> = None;
        for entry in entries.iter_mut() {
            entry.chain_valid = crate::pdf::redact::verify_chain_link(
                prev_hash.as_deref(),
                entry.previous_hash.as_deref(),
            );
            prev_hash = Some(entry.content_hash.clone());
        }
        Ok(entries)
    }

    /// Verify the entire redaction hash-chain for a source file. Returns `true`
    /// only when every link is intact (no tampering detected).
    pub fn verify_redaction_chain_integrity(&self, source_path: &str) -> Result<bool> {
        let entries = self.query_redaction_log(source_path)?;
        Ok(entries.iter().all(|e| e.chain_valid))
    }

    pub fn get_workbook_data(&self, workbook_id: i64) -> Result<WorkbookData> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt =
            conn.prepare("SELECT id, name, created_at, updated_at FROM workbooks WHERE id = ?1")?;
        let workbook = stmt.query_row(params![workbook_id], |row| {
            Ok(Workbook {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;

        let mut sheet_stmt = conn.prepare("SELECT id, workbook_id, name, sort_order, created_at FROM sheets WHERE workbook_id = ?1 ORDER BY sort_order")?;
        let sheet_rows = sheet_stmt.query_map(params![workbook_id], |row| {
            Ok(Sheet {
                id: row.get(0)?,
                workbook_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut sheets_data = Vec::new();
        for sheet in sheet_rows {
            let sheet = sheet?;
            let sheet_data = self.load_sheet_data_internal(&conn, sheet)?;
            sheets_data.push(sheet_data);
        }

        Ok(WorkbookData {
            workbook,
            sheets: sheets_data,
        })
    }

    fn load_sheet_data_internal(&self, conn: &Connection, sheet: Sheet) -> Result<SheetData> {
        let mut col_stmt = conn.prepare("SELECT id, sheet_id, name, col_type, sort_order, width FROM sheet_columns WHERE sheet_id = ?1 ORDER BY sort_order")?;
        let columns: Vec<SheetColumn> = col_stmt
            .query_map(params![sheet.id], |row| {
                Ok(SheetColumn {
                    id: row.get(0)?,
                    sheet_id: row.get(1)?,
                    name: row.get(2)?,
                    col_type: row.get(3)?,
                    sort_order: row.get(4)?,
                    width: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let mut cell_stmt = conn.prepare("SELECT id, sheet_id, row_index, column_id, value FROM cell_data WHERE sheet_id = ?1 ORDER BY row_index, column_id")?;
        let cells: Vec<CellData> = cell_stmt
            .query_map(params![sheet.id], |row| {
                Ok(CellData {
                    id: row.get(0)?,
                    sheet_id: row.get(1)?,
                    row_index: row.get(2)?,
                    column_id: row.get(3)?,
                    value: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let row_count = if columns.is_empty() {
            0
        } else {
            let mut count_stmt = conn.prepare(
                "SELECT COALESCE(MAX(row_index) + 1, 0) FROM cell_data WHERE sheet_id = ?1",
            )?;
            count_stmt.query_row(params![sheet.id], |row| row.get::<_, i64>(0))?
        };

        let mut rows: Vec<Vec<CellData>> = Vec::new();
        let mut current_row: Vec<CellData> = Vec::new();
        let mut current_row_idx: Option<i64> = None;
        for cell in cells {
            if current_row_idx != Some(cell.row_index) {
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                current_row = Vec::new();
                current_row_idx = Some(cell.row_index);
            }
            current_row.push(cell);
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        Ok(SheetData {
            sheet,
            columns,
            rows,
            row_count,
        })
    }

    pub fn create_sheet(&self, workbook_id: i64, name: &str) -> Result<Sheet> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let max_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM sheets WHERE workbook_id = ?1",
            params![workbook_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO sheets (workbook_id, name, sort_order) VALUES (?1, ?2, ?3)",
            params![workbook_id, name, max_order + 1],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE workbooks SET updated_at = datetime('now') WHERE id = ?1",
            params![workbook_id],
        )?;
        Ok(Sheet {
            id,
            workbook_id,
            name: name.to_string(),
            sort_order: max_order + 1,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn add_column(&self, sheet_id: i64, name: &str, col_type: &str) -> Result<SheetColumn> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let max_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM sheet_columns WHERE sheet_id = ?1",
            params![sheet_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO sheet_columns (sheet_id, name, col_type, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![sheet_id, name, col_type, max_order + 1],
        )?;
        let id = conn.last_insert_rowid();
        Ok(SheetColumn {
            id,
            sheet_id,
            name: name.to_string(),
            col_type: col_type.to_string(),
            sort_order: max_order + 1,
            width: 150,
        })
    }

    pub fn update_cell(
        &self,
        sheet_id: i64,
        row_index: i64,
        column_id: i64,
        value: &str,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO cell_data (sheet_id, row_index, column_id, value) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(sheet_id, row_index, column_id) DO UPDATE SET value = ?4",
            params![sheet_id, row_index, column_id, value],
        )?;
        Ok(())
    }

    pub fn add_row(&self, sheet_id: i64) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let max_row: i64 = conn.query_row(
            "SELECT COALESCE(MAX(row_index), -1) FROM cell_data WHERE sheet_id = ?1",
            params![sheet_id],
            |row| row.get(0),
        )?;
        let new_row = max_row + 1;
        let mut col_stmt = conn.prepare(
            "SELECT id, col_type FROM sheet_columns WHERE sheet_id = ?1 ORDER BY sort_order",
        )?;
        let cols: Vec<(i64, String)> = col_stmt
            .query_map(params![sheet_id], |row| {
                Ok((row.get(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>>>()?;
        for (col_id, col_type) in &cols {
            let default_val = match col_type.as_str() {
                "number" => "0",
                "boolean" => "false",
                _ => "",
            };
            conn.execute(
                "INSERT OR IGNORE INTO cell_data (sheet_id, row_index, column_id, value) VALUES (?1, ?2, ?3, ?4)",
                params![sheet_id, new_row, col_id, default_val],
            )?;
        }
        Ok(new_row)
    }

    pub fn update_workbook_name(&self, id: i64, name: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE workbooks SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn replace_sheet_data(
        &self,
        sheet_id: i64,
        columns: &[(&str, &str)],
        rows_data: &[Vec<&str>],
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "DELETE FROM cell_data WHERE sheet_id = ?1",
            params![sheet_id],
        )?;
        conn.execute(
            "DELETE FROM sheet_columns WHERE sheet_id = ?1",
            params![sheet_id],
        )?;
        let mut col_ids = Vec::new();
        for (i, (name, col_type)) in columns.iter().enumerate() {
            conn.execute(
                "INSERT INTO sheet_columns (sheet_id, name, col_type, sort_order) VALUES (?1, ?2, ?3, ?4)",
                params![sheet_id, name, col_type, i as i64],
            )?;
            col_ids.push(conn.last_insert_rowid());
        }
        for (row_idx, row_data) in rows_data.iter().enumerate() {
            for (col_idx, value) in row_data.iter().enumerate() {
                if let Some(col_id) = col_ids.get(col_idx) {
                    conn.execute(
                        "INSERT INTO cell_data (sheet_id, row_index, column_id, value) VALUES (?1, ?2, ?3, ?4)",
                        params![sheet_id, row_idx as i64, col_id, value],
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn get_business_info(&self) -> Result<Option<BusinessInfo>> {
        if let Some(cached) = self.business_info_cache.get("row") {
            return Ok(cached);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT business_name, industry, company_size, order_number_prefix, completed_onboarding FROM business_info WHERE id = 1"
        )?;
        let result = stmt.query_row([], |row| {
            Ok(BusinessInfo {
                business_name: row.get(0)?,
                industry: row.get(1)?,
                company_size: row.get(2)?,
                order_number_prefix: row.get(3)?,
                completed_onboarding: row.get::<_, i32>(4)? != 0,
            })
        });
        drop(stmt);
        drop(conn);
        let result = match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }?;
        self.business_info_cache.put("row", result.clone());
        crate::metrics::inc_db_query();
        Ok(result)
    }

    pub fn save_business_info(
        &self,
        business_name: &str,
        industry: &str,
        company_size: &str,
        order_number_prefix: &str,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT OR REPLACE INTO business_info (id, business_name, industry, company_size, order_number_prefix, completed_onboarding, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, 1, datetime('now'))",
            params![business_name, industry, company_size, order_number_prefix],
        )?;
        drop(conn);
        self.business_info_cache.invalidate_all();
        Ok(())
    }

    /// Validate the order-number prefix: empty (no prefix) or 1-4 ASCII
    /// alphanumeric chars. Returns the error message string on rejection so
    /// callers can surface it directly to the UI.
    pub fn validate_order_prefix(prefix: &str) -> Result<()> {
        if prefix.is_empty() {
            return Ok(());
        }
        if prefix.len() > 4 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if !prefix.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(())
    }

    /// Generate the next order number using the configured prefix. Strategy:
    /// 1. Read the order_number_prefix from business_info (defaulting to '').
    /// 2. Find the highest existing integer suffix across all order_numbers
    ///    that match the prefix+digits pattern, and increment by 1.
    /// 3. Fall back to a timestamp-derived number if no existing matches.
    pub fn next_order_number(&self) -> Result<String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let prefix: String = conn
            .query_row(
                "SELECT COALESCE(order_number_prefix, '') FROM business_info WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let like_pattern = format!("{}%", prefix);
        let mut stmt =
            conn.prepare("SELECT order_number FROM orders WHERE order_number LIKE ?1")?;
        let rows = stmt.query_map(params![like_pattern], |row| row.get::<_, String>(0))?;
        let mut max_n: i64 = 0;
        let mut any_match = false;
        for r in rows {
            let n = r?;
            any_match = true;
            // strip prefix, then parse the trailing digits (ignore separators)
            let tail = n.strip_prefix(prefix.as_str()).unwrap_or(&n);
            let digits: String = tail
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if let Ok(v) = digits.parse::<i64>() {
                if v > max_n {
                    max_n = v;
                }
            }
        }
        if !any_match {
            // No orders with this prefix yet — start at 1.
            return Ok(format!("{}{:04}", prefix, 1i64));
        }
        Ok(format!("{}{:04}", prefix, max_n + 1))
    }

    pub fn create_invoice(
        &self,
        invoice_number: &str,
        due_date: &str,
        payment_terms: &str,
    ) -> Result<Invoice> {
        if invoice_number.trim().is_empty() || due_date.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if invoice_number.len() > 100 || payment_terms.len() > 50 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Check if invoice number already exists
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM invoices WHERE invoice_number = ?1")?;
        let count: u32 = stmt.query_row(params![invoice_number], |row| row.get(0))?;
        if count > 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let issue_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        conn.execute(
            "INSERT INTO invoices (invoice_number, issue_date, due_date, payment_terms, status)
             VALUES (?1, ?2, ?3, ?4, 'draft')",
            params![invoice_number, issue_date, due_date, payment_terms],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Invoice {
            id,
            invoice_number: invoice_number.to_string(),
            client_id: None,
            status: "draft".to_string(),
            issue_date,
            due_date: due_date.to_string(),
            payment_terms: payment_terms.to_string(),
            subtotal: 0.0,
            tax_rate: 0.0,
            tax_amount: 0.0,
            total: 0.0,
            currency: "USD".to_string(),
            internal_notes: String::new(),
            customer_notes: String::new(),
            qb_sync_status: "not_synced".to_string(),
            amount_paid: 0.0,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn map_invoice(row: &rusqlite::Row<'_>) -> rusqlite::Result<Invoice> {
        Ok(Invoice {
            id: row.get(0)?,
            invoice_number: row.get(1)?,
            client_id: row.get(2)?,
            status: row.get(3)?,
            issue_date: row.get(4)?,
            due_date: row.get(5)?,
            payment_terms: row.get(6)?,
            subtotal: row.get(7)?,
            tax_rate: row.get(8)?,
            tax_amount: row.get(9)?,
            total: row.get(10)?,
            currency: row.get(11)?,
            internal_notes: row.get(12)?,
            customer_notes: row.get(13)?,
            qb_sync_status: row.get(14)?,
            amount_paid: row.get(15)?,
            created_at: row.get(16)?,
            updated_at: row.get(17)?,
        })
    }

    pub fn list_invoices(&self) -> Result<Vec<Invoice>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_number, client_id, status, issue_date, due_date, payment_terms,
                    subtotal, tax_rate, tax_amount, total, currency, internal_notes, customer_notes,
                    qb_sync_status, amount_paid, created_at, updated_at FROM invoices ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], Self::map_invoice)?;
        rows.collect()
    }

    pub fn list_invoices_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<crate::models::PaginatedList<Invoice>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM invoices", [], |row| row.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_number, client_id, status, issue_date, due_date, payment_terms,
                    subtotal, tax_rate, tax_amount, total, currency, internal_notes, customer_notes,
                    qb_sync_status, amount_paid, created_at, updated_at FROM invoices ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], Self::map_invoice)?;
        let collected: Vec<Invoice> = rows.collect::<Result<Vec<_>>>()?;
        Ok(crate::models::PaginatedList {
            rows: collected,
            total_count: total,
            limit,
            offset,
        })
    }

    pub fn get_invoice_data(&self, invoice_id: i64) -> Result<InvoiceData> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_number, client_id, status, issue_date, due_date, payment_terms,
                    subtotal, tax_rate, tax_amount, total, currency, internal_notes, customer_notes,
                    qb_sync_status, amount_paid, created_at, updated_at FROM invoices WHERE id = ?1"
        )?;
        let invoice = stmt.query_row(params![invoice_id], Self::map_invoice)?;

        let mut stmt = conn.prepare(
            "SELECT id, invoice_id, description, quantity, unit_price, sort_order
             FROM invoice_line_items WHERE invoice_id = ?1 ORDER BY sort_order",
        )?;
        let line_items = stmt
            .query_map(params![invoice_id], |row| {
                Ok(InvoiceLineItem {
                    id: row.get(0)?,
                    invoice_id: row.get(1)?,
                    description: row.get(2)?,
                    quantity: row.get(3)?,
                    unit_price: row.get(4)?,
                    sort_order: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(InvoiceData {
            invoice,
            line_items,
        })
    }

    pub fn add_line_item(
        &self,
        invoice_id: i64,
        description: &str,
        quantity: f64,
        unit_price: f64,
    ) -> Result<InvoiceLineItem> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let sort_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM invoice_line_items WHERE invoice_id = ?1",
            params![invoice_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO invoice_line_items (invoice_id, description, quantity, unit_price, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![invoice_id, description, quantity, unit_price, sort_order + 1],
        )?;
        let id = conn.last_insert_rowid();
        Ok(InvoiceLineItem {
            id,
            invoice_id,
            description: description.to_string(),
            quantity,
            unit_price,
            sort_order: sort_order + 1,
        })
    }

    pub fn replace_invoice_line_items(
        &self,
        invoice_id: i64,
        items: &[(String, f64, f64)],
    ) -> Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM invoice_line_items WHERE invoice_id = ?1",
            params![invoice_id],
        )?;
        for (i, (description, quantity, unit_price)) in items.iter().enumerate() {
            tx.execute(
                "INSERT INTO invoice_line_items (invoice_id, description, quantity, unit_price, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![invoice_id, description, quantity, unit_price, i as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn update_invoice(
        &self,
        id: i64,
        status: &str,
        subtotal: f64,
        tax_rate: f64,
        tax_amount: f64,
        total: f64,
        internal_notes: &str,
        customer_notes: &str,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE invoices SET status = ?1, subtotal = ?2, tax_rate = ?3, tax_amount = ?4, total = ?5, internal_notes = ?6, customer_notes = ?7, updated_at = datetime('now') WHERE id = ?8",
            params![status, subtotal, tax_rate, tax_amount, total, internal_notes, customer_notes, id],
        )?;
        Ok(())
    }

    pub fn create_order(
        &self,
        order_number: &str,
        due_date: &str,
        description: &str,
    ) -> Result<Order> {
        if order_number.trim().is_empty()
            || due_date.trim().is_empty()
            || description.trim().is_empty()
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if order_number.len() > 100 || description.len() > 1000 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO orders (order_number, due_date, description, status) VALUES (?1, ?2, ?3, 'prepress')",
            params![order_number, due_date, description],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Order {
            id,
            order_number: order_number.to_string(),
            client_id: None,
            status: "prepress".to_string(),
            priority: "normal".to_string(),
            due_date: due_date.to_string(),
            description: description.to_string(),
            artwork_notes: String::new(),
            artwork_url: None,
            artwork_approved: false,
            deposit_requested: false,
            deposit_amount: 0.0,
            total_value: 0.0,
            print_type: String::new(),
            paper_stock: String::new(),
            ink_colors: String::new(),
            finishing: String::new(),
            quantity: 0,
            production_notes: String::new(),
            assigned_operator: String::new(),
            fulfillment_method: "pickup".to_string(),
            tracking_number: String::new(),
            tracking_carrier: String::new(),
            ready_for_pickup: false,
            shipped_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn map_order(row: &rusqlite::Row<'_>) -> rusqlite::Result<Order> {
        Ok(Order {
            id: row.get(0)?,
            order_number: row.get(1)?,
            client_id: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            due_date: row.get(5)?,
            description: row.get(6)?,
            artwork_notes: row.get(7)?,
            artwork_url: row.get(8)?,
            artwork_approved: row.get::<_, i32>(9)? != 0,
            deposit_requested: row.get::<_, i32>(10)? != 0,
            deposit_amount: row.get(11)?,
            total_value: row.get(12)?,
            print_type: row.get(13)?,
            paper_stock: row.get(14)?,
            ink_colors: row.get(15)?,
            finishing: row.get(16)?,
            quantity: row.get(17)?,
            production_notes: row.get(18)?,
            assigned_operator: row.get(19)?,
            fulfillment_method: row.get(20)?,
            tracking_number: row.get(21)?,
            tracking_carrier: row.get(22)?,
            ready_for_pickup: row.get::<_, i32>(23)? != 0,
            shipped_at: row.get(24)?,
            created_at: row.get(25)?,
            updated_at: row.get(26)?,
        })
    }

    pub fn list_orders(&self) -> Result<Vec<Order>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, order_number, client_id, status, priority, due_date, description, artwork_notes, artwork_url,
                    artwork_approved, deposit_requested, deposit_amount, total_value,
                    print_type, paper_stock, ink_colors, finishing, quantity, production_notes, assigned_operator,
                    fulfillment_method, tracking_number, tracking_carrier, ready_for_pickup, shipped_at,
                    created_at, updated_at
             FROM orders ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], Self::map_order)?;
        rows.collect()
    }

    pub fn list_orders_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<crate::models::PaginatedList<Order>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT id, order_number, client_id, status, priority, due_date, description, artwork_notes, artwork_url,
                    artwork_approved, deposit_requested, deposit_amount, total_value,
                    print_type, paper_stock, ink_colors, finishing, quantity, production_notes, assigned_operator,
                    fulfillment_method, tracking_number, tracking_carrier, ready_for_pickup, shipped_at,
                    created_at, updated_at
             FROM orders ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], Self::map_order)?;
        let collected: Vec<Order> = rows.collect::<Result<Vec<_>>>()?;
        Ok(crate::models::PaginatedList {
            rows: collected,
            total_count: total,
            limit,
            offset,
        })
    }

    pub fn get_order_data(&self, order_id: i64) -> Result<OrderData> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, order_number, client_id, status, priority, due_date, description, artwork_notes, artwork_url,
                    artwork_approved, deposit_requested, deposit_amount, total_value,
                    print_type, paper_stock, ink_colors, finishing, quantity, production_notes, assigned_operator,
                    fulfillment_method, tracking_number, tracking_carrier, ready_for_pickup, shipped_at,
                    created_at, updated_at
             FROM orders WHERE id = ?1"
        )?;
        let order = stmt.query_row(params![order_id], Self::map_order)?;

        let mut stmt = conn.prepare(
            "SELECT id, order_id, previous_status, new_status, notes, created_at
             FROM order_status_history WHERE order_id = ?1 ORDER BY created_at DESC",
        )?;
        let status_history = stmt
            .query_map(params![order_id], |row| {
                Ok(OrderStatusHistory {
                    id: row.get(0)?,
                    order_id: row.get(1)?,
                    previous_status: row.get(2)?,
                    new_status: row.get(3)?,
                    notes: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(OrderData {
            order,
            status_history,
        })
    }

    pub fn update_order_status(&self, order_id: i64, new_status: &str, notes: &str) -> Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;

        // Verify order exists
        let exists: bool = tx
            .query_row(
                "SELECT COUNT(*) FROM orders WHERE id = ?1",
                params![order_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);
        if !exists {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_NOTFOUND),
                Some(format!("Order {} not found", order_id)),
            ));
        }

        let previous_status: String = tx.query_row(
            "SELECT status FROM orders WHERE id = ?1",
            params![order_id],
            |row| row.get(0),
        )?;

        // Enforce the order status state machine on the backend so a direct
        // command call cannot bypass the frontend's forward-only flow (#160).
        if !is_valid_order_transition(&previous_status, new_status) {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                Some(format!(
                    "Invalid order status transition: {} → {}",
                    previous_status, new_status
                )),
            ));
        }

        tx.execute(
            "INSERT INTO order_status_history (order_id, previous_status, new_status, notes)
             VALUES (?1, ?2, ?3, ?4)",
            params![order_id, previous_status, new_status, notes],
        )?;

        tx.execute(
            "UPDATE orders SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![new_status, order_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn update_order(
        &self,
        id: i64,
        due_date: &str,
        priority: &str,
        description: &str,
        artwork_notes: &str,
        artwork_approved: bool,
        deposit_requested: bool,
        deposit_amount: f64,
        total_value: f64,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE orders SET due_date = ?1, priority = ?2, description = ?3, artwork_notes = ?4, artwork_approved = ?5, deposit_requested = ?6, deposit_amount = ?7, total_value = ?8, updated_at = datetime('now') WHERE id = ?9",
            params![due_date, priority, description, artwork_notes, artwork_approved as i32, deposit_requested as i32, deposit_amount, total_value, id],
        )?;
        Ok(())
    }

    pub fn create_estimate(&self, estimate_number: &str, valid_until: &str) -> Result<Estimate> {
        if estimate_number.trim().is_empty() || valid_until.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if estimate_number.len() > 100 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Check if estimate number already exists
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM estimates WHERE estimate_number = ?1")?;
        let count: u32 = stmt.query_row(params![estimate_number], |row| row.get(0))?;
        if count > 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        conn.execute(
            "INSERT INTO estimates (estimate_number, valid_until, status) VALUES (?1, ?2, 'draft')",
            params![estimate_number, valid_until],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Estimate {
            id,
            estimate_number: estimate_number.to_string(),
            client_id: None,
            status: "draft".to_string(),
            valid_until: valid_until.to_string(),
            subtotal: 0.0,
            tax_rate: 0.0,
            tax_amount: 0.0,
            total: 0.0,
            currency: "USD".to_string(),
            notes: String::new(),
            artwork_requirements: String::new(),
            converted_order_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_estimates(&self) -> Result<Vec<Estimate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, estimate_number, client_id, status, valid_until, subtotal, tax_rate, tax_amount, total, currency, notes, artwork_requirements, converted_order_id, created_at, updated_at FROM estimates ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Estimate {
                id: row.get(0)?,
                estimate_number: row.get(1)?,
                client_id: row.get(2)?,
                status: row.get(3)?,
                valid_until: row.get(4)?,
                subtotal: row.get(5)?,
                tax_rate: row.get(6)?,
                tax_amount: row.get(7)?,
                total: row.get(8)?,
                currency: row.get(9)?,
                notes: row.get(10)?,
                artwork_requirements: row.get(11)?,
                converted_order_id: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_estimate_data(&self, estimate_id: i64) -> Result<EstimateData> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, estimate_number, client_id, status, valid_until, subtotal, tax_rate, tax_amount, total, currency, notes, artwork_requirements, converted_order_id, created_at, updated_at FROM estimates WHERE id = ?1"
        )?;
        let estimate = stmt.query_row(params![estimate_id], |row| {
            Ok(Estimate {
                id: row.get(0)?,
                estimate_number: row.get(1)?,
                client_id: row.get(2)?,
                status: row.get(3)?,
                valid_until: row.get(4)?,
                subtotal: row.get(5)?,
                tax_rate: row.get(6)?,
                tax_amount: row.get(7)?,
                total: row.get(8)?,
                currency: row.get(9)?,
                notes: row.get(10)?,
                artwork_requirements: row.get(11)?,
                converted_order_id: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, estimate_id, description, category, quantity, unit_price, sort_order FROM estimate_line_items WHERE estimate_id = ?1 ORDER BY sort_order"
        )?;
        let line_items = stmt
            .query_map(params![estimate_id], |row| {
                Ok(EstimateLineItem {
                    id: row.get(0)?,
                    estimate_id: row.get(1)?,
                    description: row.get(2)?,
                    category: row.get(3)?,
                    quantity: row.get(4)?,
                    unit_price: row.get(5)?,
                    sort_order: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(EstimateData {
            estimate,
            line_items,
        })
    }

    pub fn add_estimate_line_item(
        &self,
        estimate_id: i64,
        description: &str,
        category: &str,
        quantity: f64,
        unit_price: f64,
    ) -> Result<EstimateLineItem> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let sort_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM estimate_line_items WHERE estimate_id = ?1",
            params![estimate_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO estimate_line_items (estimate_id, description, category, quantity, unit_price, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![estimate_id, description, category, quantity, unit_price, sort_order + 1],
        )?;
        let id = conn.last_insert_rowid();
        Ok(EstimateLineItem {
            id,
            estimate_id,
            description: description.to_string(),
            category: category.to_string(),
            quantity,
            unit_price,
            sort_order: sort_order + 1,
        })
    }

    pub fn replace_estimate_line_items(
        &self,
        estimate_id: i64,
        items: &[(String, String, f64, f64)],
    ) -> Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM estimate_line_items WHERE estimate_id = ?1",
            params![estimate_id],
        )?;
        for (i, (description, category, quantity, unit_price)) in items.iter().enumerate() {
            tx.execute(
                "INSERT INTO estimate_line_items (estimate_id, description, category, quantity, unit_price, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![estimate_id, description, category, quantity, unit_price, i as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn update_estimate(
        &self,
        id: i64,
        status: &str,
        subtotal: f64,
        tax_rate: f64,
        tax_amount: f64,
        total: f64,
        notes: &str,
        artwork_requirements: &str,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE estimates SET status = ?1, subtotal = ?2, tax_rate = ?3, tax_amount = ?4, total = ?5, notes = ?6, artwork_requirements = ?7, updated_at = datetime('now') WHERE id = ?8",
            params![status, subtotal, tax_rate, tax_amount, total, notes, artwork_requirements, id],
        )?;
        Ok(())
    }

    pub fn add_inventory_item(
        &self,
        material_type: &str,
        size: &str,
        attributes: &str,
        quantity: f64,
        unit: &str,
        reorder_level: f64,
        alert_type: &str,
        alert_threshold: f64,
    ) -> Result<InventoryItem> {
        if material_type.trim().is_empty() || unit.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if !quantity.is_finite() || !reorder_level.is_finite() || !alert_threshold.is_finite() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if quantity < 0.0 || reorder_level < 0.0 || alert_threshold < 0.0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO inventory_items (material_type, size, attributes, quantity, unit, reorder_level, alert_type, alert_threshold) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![material_type, size, attributes, quantity, unit, reorder_level, alert_type, alert_threshold],
        )?;
        let id = conn.last_insert_rowid();
        Ok(InventoryItem {
            id,
            material_type: material_type.to_string(),
            size: size.to_string(),
            attributes: attributes.to_string(),
            quantity,
            unit: unit.to_string(),
            reorder_level,
            alert_type: alert_type.to_string(),
            alert_threshold,
            last_restocked: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_inventory_items(&self) -> Result<Vec<InventoryItem>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, material_type, size, attributes, quantity, unit, reorder_level, alert_type, alert_threshold, last_restocked, created_at, updated_at FROM inventory_items ORDER BY material_type, size LIMIT 500"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                material_type: row.get(1)?,
                size: row.get(2)?,
                attributes: row.get(3)?,
                quantity: row.get(4)?,
                unit: row.get(5)?,
                reorder_level: row.get(6)?,
                alert_type: row.get(7)?,
                alert_threshold: row.get(8)?,
                last_restocked: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_inventory_item(&self, id: i64) -> Result<InventoryItem> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, material_type, size, attributes, quantity, unit, reorder_level, alert_type, alert_threshold, last_restocked, created_at, updated_at FROM inventory_items WHERE id = ?1"
        )?;
        stmt.query_row(params![id], |row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                material_type: row.get(1)?,
                size: row.get(2)?,
                attributes: row.get(3)?,
                quantity: row.get(4)?,
                unit: row.get(5)?,
                reorder_level: row.get(6)?,
                alert_type: row.get(7)?,
                alert_threshold: row.get(8)?,
                last_restocked: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
    }

    pub fn adjust_inventory(
        &self,
        inventory_item_id: i64,
        quantity_change: f64,
        reason: &str,
        order_id: Option<i64>,
    ) -> Result<()> {
        // Wrap the whole read-then-write in a single transaction (#149).
        // Without it, two concurrent calls can both read the same quantity,
        // both pass the negative guard, then both deduct — driving the
        // quantity below zero. A transaction (BEGIN IMMEDIATE under the
        // mutex) makes the guard check atomic with the UPDATE, so the second
        // caller sees the first caller's committed value.
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        if !quantity_change.is_finite() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if quantity_change == 0.0 {
            return Ok(());
        }

        let tx = conn.transaction()?;

        // Guard: prevent quantity going below zero. Always read the current
        // value inside the tx so the check reflects any concurrent commit.
        let current: f64 = tx.query_row(
            "SELECT quantity FROM inventory_items WHERE id = ?1",
            params![inventory_item_id],
            |row| row.get(0),
        )?;
        if current + quantity_change < 0.0 {
            // Dropping `tx` without committing rolls back any pending writes.
            return Err(rusqlite::Error::InvalidQuery);
        }

        tx.execute(
            "INSERT INTO inventory_transactions (inventory_item_id, transaction_type, quantity_change, reason, related_order_id) VALUES (?1, 'adjust', ?2, ?3, ?4)",
            params![inventory_item_id, quantity_change, reason, order_id],
        )?;

        tx.execute(
            "UPDATE inventory_items SET quantity = quantity + ?1, updated_at = datetime('now') WHERE id = ?2",
            params![quantity_change, inventory_item_id],
        )?;

        let current_qty: f64 = tx.query_row(
            "SELECT quantity FROM inventory_items WHERE id = ?1",
            params![inventory_item_id],
            |row| row.get(0),
        )?;

        let (alert_type, threshold): (String, f64) = tx.query_row(
            "SELECT alert_type, alert_threshold FROM inventory_items WHERE id = ?1",
            params![inventory_item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let should_alert = match alert_type.as_str() {
            "quantity" => current_qty <= threshold,
            "percentage" => {
                let reorder: f64 = tx.query_row(
                    "SELECT reorder_level FROM inventory_items WHERE id = ?1",
                    params![inventory_item_id],
                    |row| row.get(0),
                )?;
                if reorder > 0.0 {
                    (current_qty / reorder) * 100.0 <= threshold
                } else {
                    false
                }
            }
            _ => false,
        };

        if should_alert {
            tx.execute(
                "INSERT OR IGNORE INTO inventory_alerts (inventory_item_id, alert_type, current_quantity, threshold, is_acknowledged) VALUES (?1, 'low_stock', ?2, ?3, 0)",
                params![inventory_item_id, current_qty, threshold],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_low_stock_alerts(&self) -> Result<Vec<InventoryAlert>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, inventory_item_id, alert_type, current_quantity, threshold, is_acknowledged, created_at FROM inventory_alerts WHERE is_acknowledged = 0 ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InventoryAlert {
                id: row.get(0)?,
                inventory_item_id: row.get(1)?,
                alert_type: row.get(2)?,
                current_quantity: row.get(3)?,
                threshold: row.get(4)?,
                is_acknowledged: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn acknowledge_alert(&self, alert_id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE inventory_alerts SET is_acknowledged = 1 WHERE id = ?1",
            params![alert_id],
        )?;
        Ok(())
    }

    pub fn verify_integrity(&self) -> VerificationResult {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => {
                return VerificationResult {
                    is_valid: false,
                    errors: vec!["Failed to acquire database lock".to_string()],
                    warnings: vec![],
                }
            }
        };

        let mut result = VerificationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };

        // Check foreign key constraint violations
        if let Ok(count) = conn.query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check()",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            if count > 0 {
                result.is_valid = false;
                result
                    .errors
                    .push(format!("Found {} foreign key constraint violations", count));
            }
        }

        // Check for orphaned sheet_columns (columns referencing non-existent sheets)
        if let Ok(count) = conn.query_row(
            "SELECT COUNT(*) FROM sheet_columns WHERE sheet_id NOT IN (SELECT id FROM sheets)",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            if count > 0 {
                result.is_valid = false;
                result
                    .errors
                    .push(format!("Found {} orphaned sheet_columns records", count));
            }
        }

        // Check for orphaned cell_data (cells referencing non-existent sheets or columns)
        if let Ok(count) = conn.query_row(
            "SELECT COUNT(*) FROM cell_data WHERE sheet_id NOT IN (SELECT id FROM sheets)",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            if count > 0 {
                result.is_valid = false;
                result.errors.push(format!(
                    "Found {} cell_data records with invalid sheet_id",
                    count
                ));
            }
        }

        if let Ok(count) = conn.query_row(
            "SELECT COUNT(*) FROM cell_data WHERE column_id NOT IN (SELECT id FROM sheet_columns)",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            if count > 0 {
                result.is_valid = false;
                result.errors.push(format!(
                    "Found {} cell_data records with invalid column_id",
                    count
                ));
            }
        }

        // Check for orphaned sheets (sheets referencing non-existent workbooks)
        if let Ok(count) = conn.query_row(
            "SELECT COUNT(*) FROM sheets WHERE workbook_id NOT IN (SELECT id FROM workbooks)",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            if count > 0 {
                result.is_valid = false;
                result
                    .errors
                    .push(format!("Found {} orphaned sheets records", count));
            }
        }

        // Verify required tables exist
        let required_tables = vec!["workbooks", "sheets", "sheet_columns", "cell_data"];
        for table_name in required_tables {
            if let Ok(count) = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                params![table_name],
                |row| row.get::<_, i64>(0),
            ) {
                if count == 0 {
                    result.is_valid = false;
                    result
                        .errors
                        .push(format!("Required table '{}' does not exist", table_name));
                }
            }
        }

        // Verify required indexes exist
        let required_indexes = vec!["idx_cell_data_sheet", "idx_cell_data_column"];
        for index_name in required_indexes {
            if let Ok(count) = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                params![index_name],
                |row| row.get::<_, i64>(0),
            ) {
                if count == 0 {
                    result
                        .warnings
                        .push(format!("Expected index '{}' does not exist", index_name));
                }
            }
        }

        // Run PRAGMA integrity_check
        if let Ok(mut stmt) = conn.prepare("PRAGMA integrity_check") {
            if let Ok(mut rows) = stmt.query([]) {
                while let Ok(Some(row)) = rows.next() {
                    if let Ok(message) = row.get::<_, String>(0) {
                        if message != "ok" {
                            result.is_valid = false;
                            result.errors.push(format!("Integrity check: {}", message));
                        }
                    }
                }
            }
        }

        result
    }

    // ── Clients ───────────────────────────────────────────────────────────────

    pub fn create_client(
        &self,
        name: &str,
        company: &str,
        email: &str,
        phone: &str,
        address: &str,
        tags: &str,
    ) -> Result<Client> {
        if name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO clients (name, company, email, phone, address, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name.trim(), company, email, phone, address, tags],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Client {
            id,
            name: name.trim().to_string(),
            company: company.to_string(),
            email: email.to_string(),
            phone: phone.to_string(),
            address: address.to_string(),
            tags: tags.to_string(),
            status: "active".to_string(),
            notes: String::new(),
            last_contacted: None,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub fn list_clients(
        &self,
        search: Option<&str>,
        status_filter: Option<&str>,
    ) -> Result<Vec<Client>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        const COLS: &str = "SELECT id, name, company, email, phone, address, tags, status, notes, last_contacted, created_at, updated_at FROM clients";
        match (search, status_filter) {
            (Some(s), Some(sf)) => {
                let pattern = format!("%{}%", s);
                let mut stmt = conn.prepare(&format!("{} WHERE status = ?1 AND (name LIKE ?2 OR company LIKE ?2 OR email LIKE ?2) ORDER BY name LIMIT 500", COLS))?;
                let rows = stmt
                    .query_map(params![sf, pattern], map_client)?
                    .collect::<Result<Vec<_>>>();
                rows
            }
            (Some(s), None) => {
                let pattern = format!("%{}%", s);
                let mut stmt = conn.prepare(&format!(
                    "{} WHERE name LIKE ?1 OR company LIKE ?1 OR email LIKE ?1 ORDER BY name LIMIT 500",
                    COLS
                ))?;
                let rows = stmt
                    .query_map(params![pattern], map_client)?
                    .collect::<Result<Vec<_>>>();
                rows
            }
            (None, Some(sf)) => {
                let mut stmt = conn.prepare(&format!(
                    "{} WHERE status = ?1 ORDER BY name LIMIT 500",
                    COLS
                ))?;
                let rows = stmt
                    .query_map(params![sf], map_client)?
                    .collect::<Result<Vec<_>>>();
                rows
            }
            (None, None) => {
                let mut stmt = conn.prepare(&format!("{} ORDER BY name LIMIT 500", COLS))?;
                let rows = stmt.query_map([], map_client)?.collect::<Result<Vec<_>>>();
                rows
            }
        }
    }

    pub fn list_clients_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<crate::models::PaginatedList<Client>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM clients", [], |row| row.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, company, email, phone, address, tags, status, notes, last_contacted, created_at, updated_at FROM clients ORDER BY name LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], map_client)?;
        let collected: Vec<Client> = rows.collect::<Result<Vec<_>>>()?;
        Ok(crate::models::PaginatedList {
            rows: collected,
            total_count: total,
            limit,
            offset,
        })
    }

    pub fn get_client(&self, id: i64) -> Result<Client> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.query_row(
            "SELECT id, name, company, email, phone, address, tags, status, notes, last_contacted, created_at, updated_at FROM clients WHERE id = ?1",
            params![id],
            map_client,
        )
    }

    pub fn update_client(
        &self,
        id: i64,
        name: &str,
        company: &str,
        email: &str,
        phone: &str,
        address: &str,
        tags: &str,
        status: &str,
        notes: &str,
    ) -> Result<()> {
        if name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE clients SET name=?1, company=?2, email=?3, phone=?4, address=?5, tags=?6, status=?7, notes=?8, updated_at=datetime('now') WHERE id=?9",
            params![name.trim(), company, email, phone, address, tags, status, notes, id],
        )?;
        Ok(())
    }

    pub fn delete_client(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM clients WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Art Approvals ─────────────────────────────────────────────────────────

    pub fn create_art_approval(
        &self,
        order_id: i64,
        file_path: &str,
        staff_notes: &str,
        follow_up_hours: i64,
    ) -> Result<ArtApproval> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM art_approvals WHERE order_id = ?1",
                params![order_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let token = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO art_approvals (order_id, version, file_path, staff_notes, secure_token, follow_up_hours) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![order_id, version + 1, file_path, staff_notes, token, follow_up_hours],
        )?;
        let id = conn.last_insert_rowid();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Ok(ArtApproval {
            id,
            order_id,
            version: version + 1,
            file_path: file_path.to_string(),
            status: "pending".to_string(),
            customer_notes: String::new(),
            staff_notes: staff_notes.to_string(),
            secure_token: token,
            follow_up_hours,
            follow_up_count: 0,
            submitted_at: now.clone(),
            responded_at: None,
            created_at: now,
        })
    }

    pub fn get_art_approvals_for_order(&self, order_id: i64) -> Result<Vec<ArtApproval>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, order_id, version, file_path, status, customer_notes, staff_notes, secure_token, follow_up_hours, follow_up_count, submitted_at, responded_at, created_at
             FROM art_approvals WHERE order_id = ?1 ORDER BY version DESC"
        )?;
        let rows = stmt
            .query_map(params![order_id], map_art_approval)?
            .collect::<Result<Vec<_>>>();
        rows
    }

    pub fn respond_to_art_approval(
        &self,
        token: &str,
        status: &str,
        customer_notes: &str,
    ) -> Result<ArtApproval> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let updated = conn.execute(
            "UPDATE art_approvals SET status=?1, customer_notes=?2, responded_at=datetime('now') WHERE secure_token=?3 AND status='pending'",
            params![status, customer_notes, token],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        conn.query_row(
            "SELECT id, order_id, version, file_path, status, customer_notes, staff_notes, secure_token, follow_up_hours, follow_up_count, submitted_at, responded_at, created_at FROM art_approvals WHERE secure_token=?1",
            params![token],
            map_art_approval,
        )
    }

    pub fn increment_art_approval_follow_up(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE art_approvals SET follow_up_count = follow_up_count + 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // ── Payments (#10, #11) ───────────────────────────────────────────────────

    pub fn record_payment(
        &self,
        invoice_id: Option<i64>,
        order_id: Option<i64>,
        amount: f64,
        payment_method: &str,
        reference: &str,
        notes: &str,
    ) -> Result<Payment> {
        if !amount.is_finite() || amount <= 0.0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;
        if let Some(inv_id) = invoice_id {
            let (total, status): (f64, String) = tx.query_row(
                "SELECT total, status FROM invoices WHERE id = ?1",
                params![inv_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if matches!(status.as_str(), "draft" | "voided") {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let existing: f64 = tx.query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM payments WHERE invoice_id = ?1",
                params![inv_id],
                |row| row.get(0),
            )?;
            let max_allowed = (total - existing).max(0.0) + 0.01;
            if amount > max_allowed {
                return Err(rusqlite::Error::InvalidQuery);
            }
        }
        if let Some(ord_id) = order_id {
            let total: f64 = tx.query_row(
                "SELECT total_value FROM orders WHERE id = ?1",
                params![ord_id],
                |row| row.get(0),
            )?;
            let existing: f64 = tx.query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM payments WHERE order_id = ?1",
                params![ord_id],
                |row| row.get(0),
            )?;
            let max_allowed = (total - existing).max(0.0) + 0.01;
            if amount > max_allowed {
                return Err(rusqlite::Error::InvalidQuery);
            }
        }
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        tx.execute(
            "INSERT INTO payments (invoice_id, order_id, amount, payment_method, reference, notes, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![invoice_id, order_id, amount, payment_method, reference, notes, now],
        )?;
        let id = tx.last_insert_rowid();
        // Update amount_paid on invoice if linked
        if let Some(inv_id) = invoice_id {
            let paid: f64 = tx.query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM payments WHERE invoice_id = ?1",
                params![inv_id],
                |row| row.get(0),
            )?;
            let total: f64 = tx.query_row(
                "SELECT total FROM invoices WHERE id = ?1",
                params![inv_id],
                |row| row.get(0),
            )?;
            let new_status = if paid >= total {
                "paid"
            } else {
                "partially-paid"
            };
            tx.execute(
                "UPDATE invoices SET amount_paid = ?1, status = ?2, updated_at = datetime('now') WHERE id = ?3",
                params![paid, new_status, inv_id],
            )?;
        }
        tx.commit()?;
        Ok(Payment {
            id,
            invoice_id,
            order_id,
            amount,
            payment_method: payment_method.to_string(),
            reference: reference.to_string(),
            notes: notes.to_string(),
            recorded_at: now,
        })
    }

    pub fn list_payments(
        &self,
        invoice_id: Option<i64>,
        order_id: Option<i64>,
    ) -> Result<Vec<Payment>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_id, order_id, amount, payment_method, reference, notes, recorded_at
             FROM payments WHERE (?1 IS NULL OR invoice_id = ?1) AND (?2 IS NULL OR order_id = ?2)
             ORDER BY recorded_at DESC LIMIT 500",
        )?;
        let rows = stmt
            .query_map(params![invoice_id, order_id], |row| {
                Ok(Payment {
                    id: row.get(0)?,
                    invoice_id: row.get(1)?,
                    order_id: row.get(2)?,
                    amount: row.get(3)?,
                    payment_method: row.get(4)?,
                    reference: row.get(5)?,
                    notes: row.get(6)?,
                    recorded_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>>>();
        rows
    }

    pub fn delete_payment(&self, id: i64) -> Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;
        // Re-derive amount_paid for invoice if linked
        let inv_id: Option<i64> = tx
            .query_row(
                "SELECT invoice_id FROM payments WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        tx.execute("DELETE FROM payments WHERE id = ?1", params![id])?;
        if let Some(inv_id) = inv_id {
            let paid: f64 = tx.query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM payments WHERE invoice_id = ?1",
                params![inv_id],
                |row| row.get(0),
            )?;
            let (total, current_status): (f64, String) = tx.query_row(
                "SELECT total, status FROM invoices WHERE id = ?1",
                params![inv_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            // When paid reaches 0, only promote back to "sent" if the invoice was already in
            // a payment-derived state. Preserve "draft", "overdue", "voided", etc.
            let new_status = if paid >= total && paid > 0.0 {
                "paid".to_string()
            } else if paid > 0.0 {
                "partially-paid".to_string()
            } else if current_status == "paid" || current_status == "partially-paid" {
                "sent".to_string()
            } else {
                current_status
            };
            tx.execute(
                "UPDATE invoices SET amount_paid = ?1, status = ?2, updated_at = datetime('now') WHERE id = ?3",
                params![paid, new_status, inv_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn search_invoices_and_orders(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{}%", escaped);
        let mut results: Vec<serde_json::Value> = Vec::new();
        // Search invoices
        let mut stmt = conn.prepare(
            "SELECT id, invoice_number, client_id, status, total, amount_paid FROM invoices
             WHERE invoice_number LIKE ?1 ESCAPE '\\' ORDER BY created_at DESC LIMIT 10",
        )?;
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;
        for (id, number, status, total, paid) in rows {
            results.push(serde_json::json!({ "type": "invoice", "id": id, "number": number, "status": status, "total": total, "amount_paid": paid, "balance": total - paid }));
        }
        // Search orders
        let mut stmt2 = conn.prepare(
            "SELECT id, order_number, status, total_value FROM orders
             WHERE order_number LIKE ?1 ESCAPE '\\' ORDER BY created_at DESC LIMIT 10",
        )?;
        let rows2 = stmt2
            .query_map(params![pattern], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;
        for (id, number, status, total) in rows2 {
            results.push(serde_json::json!({ "type": "order", "id": id, "number": number, "status": status, "total": total }));
        }
        Ok(results)
    }

    // ── Invoice reminders (#9) ────────────────────────────────────────────────

    pub fn log_invoice_reminder(
        &self,
        invoice_id: i64,
        method: &str,
        notes: &str,
    ) -> Result<InvoiceReminder> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "INSERT INTO invoice_reminders (invoice_id, method, notes, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![invoice_id, method, notes, now],
        )?;
        let id = conn.last_insert_rowid();
        Ok(InvoiceReminder {
            id,
            invoice_id,
            method: method.to_string(),
            notes: notes.to_string(),
            created_at: now,
        })
    }

    pub fn list_invoice_reminders(&self, invoice_id: i64) -> Result<Vec<InvoiceReminder>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_id, method, notes, created_at FROM invoice_reminders WHERE invoice_id = ?1 ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt
            .query_map(params![invoice_id], |row| {
                Ok(InvoiceReminder {
                    id: row.get(0)?,
                    invoice_id: row.get(1)?,
                    method: row.get(2)?,
                    notes: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>();
        rows
    }

    // ── QB sync (#7) ──────────────────────────────────────────────────────────

    pub fn update_invoice_qb_status(&self, id: i64, status: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE invoices SET qb_sync_status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    // ── Job specs + fulfillment (#15, #16, #18) ───────────────────────────────

    pub fn update_order_job_specs(
        &self,
        id: i64,
        print_type: &str,
        paper_stock: &str,
        ink_colors: &str,
        finishing: &str,
        quantity: i64,
        production_notes: &str,
        assigned_operator: &str,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE orders SET print_type=?1, paper_stock=?2, ink_colors=?3, finishing=?4, quantity=?5, production_notes=?6, assigned_operator=?7, updated_at=datetime('now') WHERE id=?8",
            params![print_type, paper_stock, ink_colors, finishing, quantity, production_notes, assigned_operator, id],
        )?;
        Ok(())
    }

    pub fn update_order_fulfillment(
        &self,
        id: i64,
        fulfillment_method: &str,
        tracking_number: &str,
        tracking_carrier: &str,
        ready_for_pickup: bool,
        shipped_at: Option<&str>,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE orders SET fulfillment_method=?1, tracking_number=?2, tracking_carrier=?3, ready_for_pickup=?4, shipped_at=?5, updated_at=datetime('now') WHERE id=?6",
            params![fulfillment_method, tracking_number, tracking_carrier, ready_for_pickup as i32, shipped_at, id],
        )?;
        Ok(())
    }

    // ── Department notes (#18) ────────────────────────────────────────────────

    pub fn add_department_note(
        &self,
        order_id: i64,
        note: &str,
        department: &str,
    ) -> Result<DepartmentNote> {
        if note.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "INSERT INTO department_notes (order_id, note, department, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![order_id, note.trim(), department, now],
        )?;
        let id = conn.last_insert_rowid();
        Ok(DepartmentNote {
            id,
            order_id,
            note: note.trim().to_string(),
            department: department.to_string(),
            created_at: now,
        })
    }

    pub fn list_department_notes(&self, order_id: i64) -> Result<Vec<DepartmentNote>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, order_id, note, department, created_at FROM department_notes WHERE order_id = ?1 ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt
            .query_map(params![order_id], |row| {
                Ok(DepartmentNote {
                    id: row.get(0)?,
                    order_id: row.get(1)?,
                    note: row.get(2)?,
                    department: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>();
        rows
    }

    pub fn delete_department_note(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM department_notes WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── PDF Jobs ──────────────────────────────────────────────────────────────

    pub fn save_certified_version(
        &self,
        job_id: i64,
        file_path: &str,
        file_size_bytes: u64,
        author: &str,
        comment: &str,
    ) -> Result<i64> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO pdf_certified_versions (job_id, version_number, file_path, file_size_bytes, author, comment, created_at, is_signed) \
             VALUES (?1, (SELECT COALESCE(MAX(version_number), 0) + 1 FROM pdf_certified_versions WHERE job_id = ?1), ?2, ?3, ?4, ?5, ?6, 0)",
            params![job_id, file_path, file_size_bytes as i64, author, comment, now],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    pub fn list_certified_versions(&self, job_id: i64) -> Result<Vec<CertifiedVersion>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, job_id, version_number, file_path, file_size_bytes, author, comment, created_at, is_signed \
             FROM pdf_certified_versions WHERE job_id = ?1 ORDER BY version_number DESC LIMIT 200"
        )?;
        let rows = stmt.query_map(params![job_id], |row| {
            Ok(CertifiedVersion {
                id: row.get(0)?,
                job_id: row.get(1)?,
                version_number: row.get(2)?,
                file_path: row.get(3)?,
                file_size_bytes: row.get(4)?,
                author: row.get(5)?,
                comment: row.get(6)?,
                created_at: row.get(7)?,
                is_signed: row.get(8)?,
            })
        })?;
        let mut versions = Vec::new();
        for row in rows {
            versions.push(row?);
        }
        Ok(versions)
    }

    pub fn save_pdf_job(&self, summary: &PdfSummary) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO pdf_jobs (file_path, file_name, page_count, pdf_version, file_size_bytes, title, creator, producer, is_encrypted, creation_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![summary.file_path, summary.file_name, summary.page_count as i64, summary.pdf_version, summary.file_size_bytes as i64, summary.title, summary.creator, summary.producer, summary.is_encrypted as i32, summary.creation_date],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.pdf_jobs_cache.invalidate_all();
        crate::metrics::inc_db_query();
        Ok(id)
    }

    pub fn list_pdf_jobs(&self) -> Result<Vec<PdfSummary>> {
        if let Some(cached) = self.pdf_jobs_cache.get("list") {
            return Ok(cached);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, file_path, file_name, page_count, pdf_version, file_size_bytes, title, creator, producer, is_encrypted, creation_date, opened_at FROM pdf_jobs ORDER BY opened_at DESC LIMIT 20"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PdfSummary {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                page_count: row.get::<_, i64>(3)? as usize,
                pdf_version: row.get(4)?,
                file_size_bytes: row.get::<_, i64>(5)? as u64,
                title: row.get(6)?,
                creator: row.get(7)?,
                producer: row.get(8)?,
                is_encrypted: row.get::<_, i32>(9)? != 0,
                creation_date: row.get(10)?,
            })
        })?;
        let result: Vec<PdfSummary> = rows.collect::<Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);
        self.pdf_jobs_cache.put("list", result.clone());
        crate::metrics::inc_db_query();
        Ok(result)
    }

    pub fn delete_pdf_job(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM pdf_jobs WHERE id = ?1", params![id])?;
        drop(conn);
        self.pdf_jobs_cache.invalidate_all();
        Ok(())
    }

    // ── Preflight persistence (Days 43-44) ─────────────────────────────────

    pub fn save_preflight_run(
        &self,
        job_id: i64,
        profile: &str,
        findings: &[PreflightFindingInput],
    ) -> Result<i64> {
        // #162: wrap the summary row + every finding row in a single
        // transaction so a failure partway through cannot leave a run summary
        // with no findings (or findings referencing a rolled-back summary).
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;

        let (errors, warnings, ok): (i64, i64, i64) = {
            let mut e = 0i64;
            let mut w = 0i64;
            let mut o = 0i64;
            for f in findings {
                match f.severity.as_str() {
                    "error" => e += 1,
                    "warning" => w += 1,
                    _ => o += 1,
                }
            }
            (e, w, o)
        };

        tx.execute(
            "INSERT INTO preflight_run_summary (job_id, profile, total_errors, total_warnings, total_ok) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![job_id, profile, errors, warnings, ok],
        )?;
        let run_id = tx.last_insert_rowid();

        for f in findings {
            tx.execute(
                "INSERT INTO preflight_findings (run_id, check_name, severity, page_num, object_ref, message, fix_hint) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![run_id, f.check_name, f.severity, f.page_num, f.object_ref, f.message, f.fix_hint],
            )?;
        }

        tx.commit()?;
        Ok(run_id)
    }

    pub fn list_preflight_runs(&self, job_id: i64) -> Result<Vec<PreflightRunSummary>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, job_id, profile, total_errors, total_warnings, total_ok, ran_at FROM preflight_run_summary WHERE job_id = ?1 ORDER BY ran_at DESC LIMIT 200"
        )?;
        let rows = stmt.query_map(params![job_id], |row| {
            Ok(PreflightRunSummary {
                id: row.get(0)?,
                job_id: row.get(1)?,
                profile: row.get(2)?,
                total_errors: row.get(3)?,
                total_warnings: row.get(4)?,
                total_ok: row.get(5)?,
                ran_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_findings_for_run(&self, run_id: i64) -> Result<Vec<PreflightFinding>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, run_id, check_name, severity, page_num, object_ref, message, fix_hint, created_at FROM preflight_findings WHERE run_id = ?1 ORDER BY id LIMIT 2000"
        )?;
        let rows = stmt.query_map(params![run_id], |row| {
            Ok(PreflightFinding {
                id: row.get(0)?,
                run_id: row.get(1)?,
                check_name: row.get(2)?,
                severity: row.get(3)?,
                page_num: row.get(4)?,
                object_ref: row.get(5)?,
                message: row.get(6)?,
                fix_hint: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    // ── Phase 4.1 — Preflight Profiles (#39) ──────────────────────────

    pub fn create_preflight_profile(
        &self,
        input: &PreflightProfileInput,
    ) -> Result<PreflightProfile> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO preflight_profiles (name, description, is_builtin) VALUES (?1, ?2, 0)",
            params![input.name, input.description],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.preflight_profiles_cache.invalidate_all();
        Ok(PreflightProfile {
            id,
            name: input.name.clone(),
            description: input.description.clone(),
            is_builtin: false,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub fn list_preflight_profiles(&self) -> Result<Vec<PreflightProfile>> {
        if let Some(cached) = self.preflight_profiles_cache.get("list") {
            return Ok(cached);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, is_builtin, created_at, updated_at FROM preflight_profiles ORDER BY name LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PreflightProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_builtin: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        let result: Vec<PreflightProfile> = rows.collect::<Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);
        self.preflight_profiles_cache.put("list", result.clone());
        crate::metrics::inc_db_query();
        Ok(result)
    }

    pub fn get_preflight_profile(&self, id: i64) -> Result<PreflightProfile> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.query_row(
            "SELECT id, name, description, is_builtin, created_at, updated_at FROM preflight_profiles WHERE id = ?1",
            params![id],
            |row| {
                Ok(PreflightProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    is_builtin: row.get::<_, i32>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
    }

    pub fn delete_preflight_profile(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "DELETE FROM preflight_profiles WHERE id = ?1 AND is_builtin = 0",
            params![id],
        )?;
        drop(conn);
        self.preflight_profiles_cache.invalidate_all();
        Ok(())
    }

    pub fn list_profile_checks(&self, profile_id: i64) -> Result<Vec<ProfileCheck>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, check_name, severity, enabled, params FROM profile_checks WHERE profile_id = ?1 ORDER BY check_name LIMIT 200"
        )?;
        let rows = stmt.query_map(params![profile_id], |row| {
            Ok(ProfileCheck {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                check_name: row.get(2)?,
                severity: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                params: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_profile_check(&self, check_id: i64, enabled: bool, severity: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE profile_checks SET enabled = ?1, severity = ?2 WHERE id = ?3",
            params![enabled as i32, severity, check_id],
        )?;
        drop(conn);
        self.preflight_profiles_cache.invalidate_all();
        Ok(())
    }

    pub fn list_profile_fixups(&self, profile_id: i64) -> Result<Vec<ProfileFixup>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, fixup_name, enabled, params FROM profile_fixups WHERE profile_id = ?1 ORDER BY fixup_name LIMIT 200"
        )?;
        let rows = stmt.query_map(params![profile_id], |row| {
            Ok(ProfileFixup {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                fixup_name: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                params: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_profile_fixup(&self, fixup_id: i64, enabled: bool, params: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE profile_fixups SET enabled = ?1, params = ?2 WHERE id = ?3",
            params![enabled as i32, params, fixup_id],
        )?;
        drop(conn);
        self.preflight_profiles_cache.invalidate_all();
        Ok(())
    }

    // ── Phase 4.2 — Action Lists (#38) ─────────────────────────────────

    pub fn create_action_list(&self, input: &ActionListInput) -> Result<ActionList> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO action_lists (name, description) VALUES (?1, ?2)",
            params![input.name, input.description],
        )?;
        let id = conn.last_insert_rowid();
        Ok(ActionList {
            id,
            name: input.name.clone(),
            description: input.description.clone(),
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub fn list_action_lists(&self) -> Result<Vec<ActionList>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at FROM action_lists ORDER BY name LIMIT 200",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ActionList {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_action_list(&self, id: i64) -> Result<ActionList> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.query_row(
            "SELECT id, name, description, created_at, updated_at FROM action_lists WHERE id = ?1",
            params![id],
            |row| {
                Ok(ActionList {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
    }

    pub fn delete_action_list(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM action_lists WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_action_list_step(
        &self,
        action_list_id: i64,
        input: &ActionListStepInput,
    ) -> Result<ActionListStep> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let max_order: i64 = conn.query_row(
            "SELECT COALESCE(MAX(step_order), -1) FROM action_list_steps WHERE action_list_id = ?1",
            params![action_list_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO action_list_steps (action_list_id, step_order, action_type, params) VALUES (?1, ?2, ?3, ?4)",
            params![action_list_id, max_order + 1, input.action_type, input.params],
        )?;
        let id = conn.last_insert_rowid();
        Ok(ActionListStep {
            id,
            action_list_id,
            step_order: max_order + 1,
            action_type: input.action_type.clone(),
            params: input.params.clone(),
        })
    }

    pub fn list_action_list_steps(&self, action_list_id: i64) -> Result<Vec<ActionListStep>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, action_list_id, step_order, action_type, params FROM action_list_steps WHERE action_list_id = ?1 ORDER BY step_order LIMIT 200"
        )?;
        let rows = stmt.query_map(params![action_list_id], |row| {
            Ok(ActionListStep {
                id: row.get(0)?,
                action_list_id: row.get(1)?,
                step_order: row.get(2)?,
                action_type: row.get(3)?,
                params: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_action_list_step(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM action_list_steps WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn reorder_action_list_steps(&self, action_list_id: i64, step_ids: &[i64]) -> Result<()> {
        // #167: apply all step_order updates atomically so a failure mid-loop
        // cannot leave the action list in a partially-reordered state.
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;
        for (i, step_id) in step_ids.iter().enumerate() {
            tx.execute(
                "UPDATE action_list_steps SET step_order = ?1 WHERE id = ?2 AND action_list_id = ?3",
                params![i as i64, step_id, action_list_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // ── Phase 4.3 — Batch Processing (#40) ─────────────────────────────

    pub fn create_batch_job(&self, action_list_id: i64, files: &[String]) -> Result<BatchJob> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO batch_jobs (action_list_id, status, total_files, processed_files, error_count) VALUES (?1, 'pending', ?2, 0, 0)",
            params![action_list_id, files.len() as i64],
        )?;
        let batch_id = tx.last_insert_rowid();
        for file_path in files {
            tx.execute(
                "INSERT INTO batch_results (batch_id, file_path, status) VALUES (?1, ?2, 'pending')",
                params![batch_id, file_path],
            )?;
        }
        let created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        tx.commit()?;
        Ok(BatchJob {
            id: batch_id,
            action_list_id,
            status: "pending".to_string(),
            total_files: files.len() as i64,
            processed_files: 0,
            error_count: 0,
            started_at: None,
            completed_at: None,
            created_at,
        })
    }

    pub fn list_batch_jobs(&self) -> Result<Vec<BatchJob>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, action_list_id, status, total_files, processed_files, error_count, started_at, completed_at, created_at FROM batch_jobs ORDER BY created_at DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BatchJob {
                id: row.get(0)?,
                action_list_id: row.get(1)?,
                status: row.get(2)?,
                total_files: row.get(3)?,
                processed_files: row.get(4)?,
                error_count: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_batch_job(&self, id: i64) -> Result<BatchJob> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.query_row(
            "SELECT id, action_list_id, status, total_files, processed_files, error_count, started_at, completed_at, created_at FROM batch_jobs WHERE id = ?1",
            params![id],
            |row| {
                Ok(BatchJob {
                    id: row.get(0)?,
                    action_list_id: row.get(1)?,
                    status: row.get(2)?,
                    total_files: row.get(3)?,
                    processed_files: row.get(4)?,
                    error_count: row.get(5)?,
                    started_at: row.get(6)?,
                    completed_at: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )
    }

    pub fn run_batch(&self, batch_id: i64) -> Result<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let tx = conn.transaction()?;

        let pending_results: Vec<(i64, String)> = {
            let mut stmt = tx.prepare(
                "SELECT id, file_path FROM batch_results WHERE batch_id = ?1 AND status = 'pending' ORDER BY id LIMIT 2000"
            )?;
            let x = stmt
                .query_map(params![batch_id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>>>()?;
            x
        };

        if pending_results.is_empty() {
            tx.commit()?;
            return Ok(());
        }

        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut passed = 0i64;
        let mut failed = 0i64;

        for (result_id, file_path) in pending_results {
            let input = std::path::PathBuf::from(&file_path);
            let status: &str;
            let output_path: Option<String>;
            let error_message: Option<String>;

            if !input.exists() {
                status = "failed";
                output_path = None;
                error_message = Some(format!("File not found: {}", file_path));
                failed += 1;
            } else {
                let out = input.with_extension("processed.pdf");
                match std::fs::copy(&input, &out) {
                    Ok(_) => {
                        status = "completed";
                        output_path = Some(out.to_string_lossy().to_string());
                        error_message = None;
                        passed += 1;
                    }
                    Err(e) => {
                        status = "failed";
                        output_path = None;
                        error_message = Some(format!("Copy failed: {}", e));
                        failed += 1;
                    }
                }
            }

            tx.execute(
                "UPDATE batch_results SET status = ?1, output_path = ?2, error_message = ?3, started_at = ?4, completed_at = ?5 WHERE id = ?6",
                params![status, output_path, error_message, &now, &now, result_id],
            )?;
        }

        tx.execute(
            "UPDATE batch_jobs SET status = ?1, processed_files = processed_files + ?2, error_count = error_count + ?3, completed_at = ?4 WHERE id = ?5",
            params!["completed", passed + failed, failed, &now, batch_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn list_batch_results(&self, batch_id: i64) -> Result<Vec<BatchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, batch_id, file_path, status, output_path, error_message, started_at, completed_at FROM batch_results WHERE batch_id = ?1 ORDER BY id LIMIT 2000"
        )?;
        let rows = stmt.query_map(params![batch_id], |row| {
            Ok(BatchResult {
                id: row.get(0)?,
                batch_id: row.get(1)?,
                file_path: row.get(2)?,
                status: row.get(3)?,
                output_path: row.get(4)?,
                error_message: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    // ── Phase 4.5 — Hot Folders (#42) ──────────────────────────────────

    pub fn create_hot_folder(&self, input: &HotFolderInput) -> Result<HotFolder> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO hot_folders (name, watch_path, action_list_id, output_path, file_pattern, is_active) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![input.name, input.watch_path, input.action_list_id, input.output_path, input.file_pattern],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.hot_folders_cache.invalidate_all();
        Ok(HotFolder {
            id,
            name: input.name.clone(),
            watch_path: input.watch_path.clone(),
            action_list_id: input.action_list_id,
            output_path: input.output_path.clone(),
            file_pattern: input.file_pattern.clone(),
            is_active: true,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub fn list_hot_folders(&self) -> Result<Vec<HotFolder>> {
        if let Some(cached) = self.hot_folders_cache.get("list") {
            return Ok(cached);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, name, watch_path, action_list_id, output_path, file_pattern, is_active, created_at, updated_at FROM hot_folders ORDER BY name LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HotFolder {
                id: row.get(0)?,
                name: row.get(1)?,
                watch_path: row.get(2)?,
                action_list_id: row.get(3)?,
                output_path: row.get(4)?,
                file_pattern: row.get(5)?,
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let result: Vec<HotFolder> = rows.collect::<Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);
        self.hot_folders_cache.put("list", result.clone());
        crate::metrics::inc_db_query();
        Ok(result)
    }

    pub fn delete_hot_folder(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM hot_folders WHERE id = ?1", params![id])?;
        drop(conn);
        self.hot_folders_cache.invalidate_all();
        Ok(())
    }

    pub fn toggle_hot_folder(&self, id: i64, is_active: bool) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE hot_folders SET is_active = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![is_active as i32, id],
        )?;
        drop(conn);
        self.hot_folders_cache.invalidate_all();
        Ok(())
    }

    // ── Phase 5.3 — Analytics (#50) ────────────────────────────────────

    /// Per-client pass-rate analytics. Returns one row per client with
    /// their total preflight runs, error count, and pass rate as a
    /// fraction in [0, 1]. Pass rate is derived from the
    /// `preflight_run_summary` table joined to `orders` and `clients`.
    pub fn get_client_pass_rates(&self) -> Result<Vec<ClientPassRate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = match conn.prepare(
            "SELECT COALESCE(c.name, 'Unassigned') AS client_name,
                    COUNT(pr.id) AS runs,
                    COALESCE(SUM(pr.total_errors), 0) AS errors,
                    COALESCE(SUM(pr.total_warnings), 0) AS warnings
             FROM preflight_run_summary pr
             LEFT JOIN orders o ON pr.job_id = o.id
             LEFT JOIN clients c ON o.client_id = c.id
             GROUP BY client_name
             ORDER BY runs DESC
             LIMIT 100",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt
            .query_map([], |row| {
                let runs: i64 = row.get(1)?;
                let errors: i64 = row.get(2)?;
                let pass_rate = if runs > 0 {
                    (runs - errors) as f64 / runs as f64
                } else {
                    0.0
                };
                Ok(ClientPassRate {
                    client_name: row.get(0)?,
                    runs,
                    errors,
                    warnings: row.get(3)?,
                    pass_rate,
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
    }

    /// Average turnaround (in hours) for orders that have both a
    /// created_at and a shipped_at timestamp. Returns 0.0 when no
    /// orders have shipped yet.
    pub fn get_average_turnaround_hours(&self) -> Result<f64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let avg_hours: Option<f64> = conn
            .query_row(
                "SELECT AVG((julianday(shipped_at) - julianday(created_at)) * 24.0)
                 FROM orders
                 WHERE shipped_at IS NOT NULL
                   AND created_at IS NOT NULL
                   AND shipped_at >= created_at",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        Ok(avg_hours.unwrap_or(0.0))
    }

    /// Common error categories ranked by frequency, capped at 25.
    /// Pulls from `preflight_findings` where severity = 'error'.
    pub fn get_common_error_categories(&self) -> Result<Vec<(String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = match conn.prepare(
            "SELECT check_name, COUNT(*) AS cnt
             FROM preflight_findings
             WHERE severity = 'error'
             GROUP BY check_name
             ORDER BY cnt DESC
             LIMIT 25",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
    }

    pub fn get_analytics_summary(&self) -> Result<AnalyticsSummary> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let total_jobs: i64 = conn
            .query_row("SELECT COUNT(*) FROM pdf_jobs", [], |row| row.get(0))
            .unwrap_or(0);
        let total_preflight_runs: i64 = conn
            .query_row("SELECT COUNT(*) FROM preflight_run_summary", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);
        let total_errors: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(total_errors), 0) FROM preflight_run_summary",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let total_warnings: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(total_warnings), 0) FROM preflight_run_summary",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let most_common_errors: Vec<(String, i64)> = match conn.prepare(
            "SELECT check_name, COUNT(*) as cnt FROM preflight_findings WHERE severity = 'error' GROUP BY check_name ORDER BY cnt DESC LIMIT 10"
        ) {
            Ok(mut stmt) => match stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            }) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        };
        let jobs_by_day: Vec<(String, i64)> = match conn.prepare(
            "SELECT DATE(opened_at) as day, COUNT(*) as cnt FROM pdf_jobs GROUP BY day ORDER BY day DESC LIMIT 30"
        ) {
            Ok(mut stmt) => match stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            }) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        };
        Ok(AnalyticsSummary {
            total_jobs,
            total_preflight_runs,
            total_errors,
            total_warnings,
            most_common_errors,
            jobs_by_day,
        })
    }

    // ── Phase 6.1 — Email / FTP / Webhook (#54, #52) ───────────────────

    pub fn save_email_settings(&self, settings: &EmailSettings) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        // Store password in OS keychain instead of SQLite
        if !settings.smtp_password.is_empty() {
            let _ = crate::keychain::write_secret(
                "frappe-email",
                "smtp_password",
                &settings.smtp_password,
            );
        }
        conn.execute(
            "INSERT OR REPLACE INTO email_settings (id, smtp_host, smtp_port, smtp_username, smtp_password, use_tls, from_address) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![settings.smtp_host, settings.smtp_port as i64, settings.smtp_username, "", settings.use_tls as i32, settings.from_address],
        )?;
        Ok(())
    }

    pub fn get_email_settings(&self) -> Result<Option<EmailSettings>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let result = conn.query_row(
            "SELECT smtp_host, smtp_port, smtp_username, use_tls, from_address FROM email_settings WHERE id = 1",
            [],
            |row| {
                Ok(EmailSettings {
                    smtp_host: row.get(0)?,
                    smtp_port: row.get::<_, i64>(1)? as u16,
                    smtp_username: row.get(2)?,
                    smtp_password: String::new(),
                    use_tls: row.get::<_, i32>(3)? != 0,
                    from_address: row.get(4)?,
                })
            },
        );
        match result {
            Ok(mut s) => {
                // Read password from keychain
                if let Ok(secret) = crate::keychain::read_secret("frappe-email", "smtp_password") {
                    if let Some(value) = secret.value {
                        s.smtp_password = value;
                    }
                }
                Ok(Some(s))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn save_ftp_settings(&self, settings: &FtpSettings) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        // Store password in OS keychain instead of SQLite
        if !settings.password.is_empty() {
            let _ = crate::keychain::write_secret("frappe-ftp", "password", &settings.password);
        }
        conn.execute(
            "INSERT OR REPLACE INTO ftp_settings (id, host, port, username, password, remote_dir) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![settings.host, settings.port as i64, settings.username, "", settings.remote_dir],
        )?;
        Ok(())
    }

    pub fn get_ftp_settings(&self) -> Result<Option<FtpSettings>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let result = conn.query_row(
            "SELECT host, port, username, remote_dir FROM ftp_settings WHERE id = 1",
            [],
            |row| {
                Ok(FtpSettings {
                    host: row.get(0)?,
                    port: row.get::<_, i64>(1)? as u16,
                    username: row.get(2)?,
                    password: String::new(),
                    remote_dir: row.get(3)?,
                })
            },
        );
        match result {
            Ok(mut s) => {
                // Read password from keychain
                if let Ok(secret) = crate::keychain::read_secret("frappe-ftp", "password") {
                    if let Some(value) = secret.value {
                        s.password = value;
                    }
                }
                Ok(Some(s))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn create_webhook(&self, url: &str, event: &str) -> Result<WebhookEntry> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO webhooks (url, event, is_active) VALUES (?1, ?2, 1)",
            params![url, event],
        )?;
        let id = conn.last_insert_rowid();
        Ok(WebhookEntry {
            id,
            url: url.to_string(),
            event: event.to_string(),
            is_active: true,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub fn list_webhooks(&self) -> Result<Vec<WebhookEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, url, event, is_active, created_at FROM webhooks ORDER BY created_at DESC LIMIT 200",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WebhookEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                event: row.get(2)?,
                is_active: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_webhook(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM webhooks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Event log (#83) ──────────────────────────────────────────────────

    #[allow(dead_code)] // Wired in #133 — see lib.rs upload command surface
    pub fn log_event(
        &self,
        tenant_id: &str,
        entity_type: &str,
        entity_id: i64,
        action: &str,
        payload: &str,
    ) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO event_log (tenant_id, entity_type, entity_id, action, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![tenant_id, entity_type, entity_id, action, payload],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_events(
        &self,
        tenant_id: &str,
        entity_type: Option<&str>,
        entity_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<EventLogEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut sql = "SELECT id, tenant_id, entity_type, entity_id, action, payload, created_at FROM event_log WHERE tenant_id = ?".to_string();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(tenant_id.to_string())];
        if let Some(et) = entity_type {
            sql.push_str(" AND entity_type = ?");
            param_values.push(Box::new(et.to_string()));
        }
        if let Some(eid) = entity_id {
            sql.push_str(" AND entity_id = ?");
            param_values.push(Box::new(eid));
        }
        sql.push_str(" ORDER BY id DESC LIMIT ?");
        param_values.push(Box::new(limit));
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(EventLogEntry {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                action: row.get(4)?,
                payload: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    // ── Preferences (#241, #275) ──────────────────────────────────────────

    pub fn get_preference(&self, key: &str) -> Result<Option<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let result = conn.query_row(
            "SELECT value FROM preferences WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT OR REPLACE INTO preferences (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_preferences(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare("SELECT key, value FROM preferences")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            out.insert(k, v);
        }
        Ok(out)
    }

    // ── Alt text (#234) ─────────────────────────────────────────────

    pub fn get_alt_text(
        &self,
        file_path: &str,
        object_id: i64,
    ) -> Result<Option<(String, bool)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let result = conn.query_row(
            "SELECT alt_text, is_decorative FROM image_alt_text WHERE file_path = ?1 AND object_id = ?2",
            params![file_path, object_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? != 0)),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn get_alt_text_for_file(&self, file_path: &str) -> Result<Vec<(i64, String, bool)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT object_id, alt_text, is_decorative FROM image_alt_text WHERE file_path = ?1 ORDER BY object_id",
        )?;
        let rows = stmt.query_map(params![file_path], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)? != 0,
            ))
        })?;
        rows.collect()
    }

    pub fn set_alt_text(
        &self,
        file_path: &str,
        object_id: i64,
        alt_text: &str,
        is_decorative: bool,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT OR REPLACE INTO image_alt_text (file_path, object_id, alt_text, is_decorative, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![file_path, object_id, alt_text, is_decorative as i32],
        )?;
        Ok(())
    }

    // ── PDF Annotations (#230) ──────────────────────────────────────────

    pub fn add_annotation(
        &self,
        file_path: &str,
        page: i64,
        annotation_type: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: &str,
        content: &str,
    ) -> Result<PdfAnnotation> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO pdf_annotations (file_path, page, annotation_type, x, y, width, height, color, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![file_path, page, annotation_type, x, y, width, height, color, content],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, file_path, page, annotation_type, x, y, width, height, color, content, created_at, updated_at FROM pdf_annotations WHERE id = ?1",
            params![id],
            |row| Ok(PdfAnnotation {
                id: row.get(0)?,
                file_path: row.get(1)?,
                page: row.get(2)?,
                annotation_type: row.get(3)?,
                x: row.get(4)?,
                y: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                color: row.get(8)?,
                content: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            }),
        )
    }

    pub fn list_annotations(&self, file_path: &str) -> Result<Vec<PdfAnnotation>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, file_path, page, annotation_type, x, y, width, height, color, content, created_at, updated_at FROM pdf_annotations WHERE file_path = ?1 ORDER BY page, created_at",
        )?;
        let rows = stmt.query_map(params![file_path], |row| {
            Ok(PdfAnnotation {
                id: row.get(0)?,
                file_path: row.get(1)?,
                page: row.get(2)?,
                annotation_type: row.get(3)?,
                x: row.get(4)?,
                y: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                color: row.get(8)?,
                content: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_annotation(
        &self,
        id: i64,
        color: Option<&str>,
        content: Option<&str>,
    ) -> Result<PdfAnnotation> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if let Some(c) = color {
            conn.execute(
                "UPDATE pdf_annotations SET color = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![c, id],
            )?;
        }
        if let Some(t) = content {
            conn.execute(
                "UPDATE pdf_annotations SET content = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![t, id],
            )?;
        }
        conn.query_row(
            "SELECT id, file_path, page, annotation_type, x, y, width, height, color, content, created_at, updated_at FROM pdf_annotations WHERE id = ?1",
            params![id],
            |row| Ok(PdfAnnotation {
                id: row.get(0)?,
                file_path: row.get(1)?,
                page: row.get(2)?,
                annotation_type: row.get(3)?,
                x: row.get(4)?,
                y: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                color: row.get(8)?,
                content: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            }),
        )
    }

    pub fn delete_annotation(&self, id: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM pdf_annotations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn annotation_page_counts(
        &self,
        file_path: &str,
    ) -> Result<std::collections::HashMap<i64, i64>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT page, COUNT(*) FROM pdf_annotations WHERE file_path = ?1 GROUP BY page",
        )?;
        let rows = stmt.query_map(params![file_path], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (page, count) = row?;
            map.insert(page, count);
        }
        Ok(map)
    }

    pub fn add_annotation_reply(
        &self,
        annotation_id: i64,
        content: &str,
    ) -> Result<PdfAnnotationReply> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO pdf_annotation_replies (annotation_id, content) VALUES (?1, ?2)",
            params![annotation_id, content],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, annotation_id, content, created_at FROM pdf_annotation_replies WHERE id = ?1",
            params![id],
            |row| Ok(PdfAnnotationReply {
                id: row.get(0)?,
                annotation_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            }),
        )
    }

    pub fn list_annotation_replies(
        &self,
        annotation_id: i64,
    ) -> Result<Vec<PdfAnnotationReply>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, annotation_id, content, created_at FROM pdf_annotation_replies WHERE annotation_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![annotation_id], |row| {
            Ok(PdfAnnotationReply {
                id: row.get(0)?,
                annotation_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    // ── Schema versioning (#90) ──────────────────────────────────────────

    pub fn get_schema_version(&self) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get::<_, i64>(0)
        })
    }

    #[allow(dead_code)] // Future schema-versioning API; migrations are currently applied inline in `new()`
    pub fn migrate_to(&self, target_version: i64) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let current: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )?;
        if current >= target_version {
            return Ok(());
        }
        conn.execute(
            "INSERT INTO schema_version (version, migrated_at) VALUES (?1, datetime('now'))",
            params![target_version],
        )?;
        Ok(())
    }

    // ── Backup / Restore (#90) ───────────────────────────────────────────

    /// Export a plaintext (unencrypted) copy of the database to the given path.
    /// Uses sqlcipher_export() via ATTACH with compatible plaintext settings.
    #[cfg(feature = "sqlcipher")]
    pub fn export_plaintext_backup(&self, output_path: &std::path::Path) -> Result<u64> {
        let _key_hex = self.key.as_hex();
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Remove existing file if present
        if output_path.exists() {
            std::fs::remove_file(output_path).map_err(|e| {
                let msg = format!("failed to remove existing backup: {}", e);
                rusqlite::Error::ToSqlConversionFailure(msg.into())
            })?;
        }

        // ATTACH a new plaintext database at the output path.
        // Validate the path to prevent SQL injection via the ATTACH string literal.
        let out_str = validate_sql_path(output_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        conn.execute_batch(&format!(
            "ATTACH DATABASE '{}' AS plaintext_backup KEY '';
             PRAGMA plaintext_backup.cipher_use_hmac = OFF;
             PRAGMA plaintext_backup.kdf_iter = 0;
             PRAGMA plaintext_backup.cipher_page_size = 1024;",
            out_str
        ))?;
        conn.execute_batch("SELECT sqlcipher_export('plaintext_backup');")?;
        conn.execute_batch("DETACH DATABASE plaintext_backup;")?;

        let meta = std::fs::metadata(output_path).map_err(|e| {
            let msg = format!("failed to stat backup: {}", e);
            rusqlite::Error::ToSqlConversionFailure(msg.into())
        })?;
        Ok(meta.len())
    }

    /// Plain SQLite (no SQLCipher) — just copy the file directly.
    #[cfg(not(feature = "sqlcipher"))]
    pub fn export_plaintext_backup(&self, output_path: &std::path::Path) -> Result<u64> {
        // Use SQLite's built-in VACUUM INTO for a clean, defragmented copy
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if output_path.exists() {
            std::fs::remove_file(output_path).map_err(|e| {
                let msg = format!("failed to remove existing backup: {}", e);
                rusqlite::Error::ToSqlConversionFailure(msg.into())
            })?;
        }
        let path_str = output_path.to_str().ok_or_else(|| {
            let msg = "invalid backup path".to_string();
            rusqlite::Error::ToSqlConversionFailure(msg.into())
        })?;
        conn.execute("VACUUM INTO ?1", params![path_str])?;
        let meta = std::fs::metadata(output_path).map_err(|e| {
            let msg = format!("failed to stat backup: {}", e);
            rusqlite::Error::ToSqlConversionFailure(msg.into())
        })?;
        Ok(meta.len())
    }

    pub fn create_backup(&self, backup_path: &std::path::Path) -> Result<BackupEntry> {
        // VACUUM INTO can take seconds/minutes on a large DB. Running it
        // while holding `self.conn.lock()` would freeze the entire UI because
        // every other DB call blocks on that mutex (#148). Instead open a
        // SEPARATE connection to the same database file just for the backup.
        // In WAL mode readers (and VACUUM INTO) don't block the main writer,
        // so this connection can chug away without stalling other callers.
        let backup_conn = Connection::open(&self.db_path)?;

        // When SQLCipher at-rest encryption is enabled, the new connection
        // must supply the same key before it can read anything. Use the same
        // per-PRAGMA execute_batch form as the open path (see #146).
        #[cfg(feature = "sqlcipher")]
        {
            let key_hex = self.key.as_hex();
            backup_conn.execute_batch(&format!("PRAGMA key = \"x'{}'\"", key_hex))?;
            backup_conn.execute_batch("PRAGMA cipher_compatibility = 4")?;
            backup_conn.execute_batch("PRAGMA cipher_page_size = 4096")?;
        }

        let path_str = backup_path.to_string_lossy().to_string();
        backup_conn.execute("VACUUM INTO ?1", params![path_str])?;
        // Drop the backup connection explicitly so the file is released before
        // we stat it (purely defensive; not strictly required).
        drop(backup_conn);

        let meta = std::fs::metadata(backup_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let size = meta.len() as i64;

        // Only now — for the brief INSERT that records the backup row — do we
        // acquire the main connection's mutex. This keeps the critical section
        // tiny so other callers aren't blocked.
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO backup_entries (backup_type, file_path, size_bytes) VALUES ('snapshot', ?1, ?2)",
            params![backup_path.to_string_lossy().to_string(), size],
        )?;
        let id = conn.last_insert_rowid();
        Ok(BackupEntry {
            id,
            backup_type: "snapshot".into(),
            file_path: backup_path.to_string_lossy().to_string(),
            size_bytes: size,
            checksum: String::new(),
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    pub fn list_backups(&self) -> Result<Vec<BackupEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, backup_type, file_path, size_bytes, checksum, created_at FROM backup_entries ORDER BY id DESC LIMIT 200"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupEntry {
                id: row.get(0)?,
                backup_type: row.get(1)?,
                file_path: row.get(2)?,
                size_bytes: row.get(3)?,
                checksum: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }
}

fn map_client(row: &rusqlite::Row) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get(0)?,
        name: row.get(1)?,
        company: row.get(2)?,
        email: row.get(3)?,
        phone: row.get(4)?,
        address: row.get(5)?,
        tags: row.get(6)?,
        status: row.get(7)?,
        notes: row.get(8)?,
        last_contacted: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn map_art_approval(row: &rusqlite::Row) -> rusqlite::Result<ArtApproval> {
    Ok(ArtApproval {
        id: row.get(0)?,
        order_id: row.get(1)?,
        version: row.get(2)?,
        file_path: row.get(3)?,
        status: row.get(4)?,
        customer_notes: row.get(5)?,
        staff_notes: row.get(6)?,
        secure_token: row.get(7)?,
        follow_up_hours: row.get(8)?,
        follow_up_count: row.get(9)?,
        submitted_at: row.get(10)?,
        responded_at: row.get(11)?,
        created_at: row.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    /// #249 — invariant: every list_* function in db.rs must have a LIMIT
    /// clause. The static check guards against a future refactor that
    /// accidentally removes the cap and re-introduces the unbounded-SELECT
    /// perf cliff. Names must match the public `pub fn list_*` methods.
    #[test]
    fn limit_clause_invariant() {
        let src = include_str!("db.rs");
        let required: &[(&str, &str)] = &[
            ("list_invoices", "LIMIT 200"),
            ("list_orders", "LIMIT 200"),
            ("list_estimates", "LIMIT 200"),
            ("list_inventory_items", "LIMIT 500"),
            ("list_clients", "LIMIT 500"),
        ];
        for (fn_name, limit_token) in required {
            let needle = format!("pub fn {fn_name}");
            let start = src
                .find(&needle)
                .unwrap_or_else(|| panic!("{fn_name} not found"));
            // Each list function is short (well under 2 KiB); scanning a fixed
            // window after the declaration catches the LIMIT inside the
            // prepare() call without depending on the exact brace matching.
            let end = (start + 2048).min(src.len());
            let slice = &src[start..end];
            assert!(
                slice.contains(limit_token),
                "{fn_name} missing {limit_token} in its query"
            );
        }
    }

    /// #160 — the backend order status machine must accept only the
    /// forward-only flow and reject everything else, including backward
    /// moves and arbitrary strings injected via a direct command call.
    #[test]
    fn order_status_transitions() {
        use super::is_valid_order_transition as ok;

        // Valid forward transitions.
        assert!(ok("prepress", "production"));
        assert!(ok("production", "delivery"));
        assert!(ok("delivery", "completed"));

        // No-op (idempotent re-submit) is allowed.
        assert!(ok("prepress", "prepress"));
        assert!(ok("completed", "completed"));

        // Backward moves are rejected.
        assert!(!ok("production", "prepress"));
        assert!(!ok("completed", "delivery"));

        // Skipping stages is rejected.
        assert!(!ok("prepress", "delivery"));
        assert!(!ok("prepress", "completed"));

        // Terminal state has no outgoing transitions.
        assert!(!ok("completed", "production"));

        // Arbitrary / injected strings are rejected.
        assert!(!ok("prepress", "deleted"));
        assert!(!ok("anything", "production"));
    }
}
