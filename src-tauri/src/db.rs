use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::*;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("frappe.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Database { conn: Mutex::new(conn) };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS business_info (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                business_name TEXT,
                industry TEXT,
                company_size TEXT,
                completed_onboarding INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS workbooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
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
            CREATE INDEX IF NOT EXISTS idx_cell_data_column ON cell_data(sheet_id, column_id);"
        )?;
        Ok(())
    }

    pub fn create_workbook(&self, name: &str) -> Result<Workbook> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare("SELECT id, name, created_at, updated_at FROM workbooks ORDER BY updated_at DESC")?;
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM workbooks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_workbook_data(&self, workbook_id: i64) -> Result<WorkbookData> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare("SELECT id, name, created_at, updated_at FROM workbooks WHERE id = ?1")?;
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

        Ok(WorkbookData { workbook, sheets: sheets_data })
    }

    fn load_sheet_data_internal(&self, conn: &Connection, sheet: Sheet) -> Result<SheetData> {
        let mut col_stmt = conn.prepare("SELECT id, sheet_id, name, col_type, sort_order, width FROM sheet_columns WHERE sheet_id = ?1 ORDER BY sort_order")?;
        let columns: Vec<SheetColumn> = col_stmt.query_map(params![sheet.id], |row| {
            Ok(SheetColumn {
                id: row.get(0)?,
                sheet_id: row.get(1)?,
                name: row.get(2)?,
                col_type: row.get(3)?,
                sort_order: row.get(4)?,
                width: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        let mut cell_stmt = conn.prepare("SELECT id, sheet_id, row_index, column_id, value FROM cell_data WHERE sheet_id = ?1 ORDER BY row_index, column_id")?;
        let cells: Vec<CellData> = cell_stmt.query_map(params![sheet.id], |row| {
            Ok(CellData {
                id: row.get(0)?,
                sheet_id: row.get(1)?,
                row_index: row.get(2)?,
                column_id: row.get(3)?,
                value: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        let row_count = if columns.is_empty() { 0 } else {
            let mut count_stmt = conn.prepare("SELECT COALESCE(MAX(row_index) + 1, 0) FROM cell_data WHERE sheet_id = ?1")?;
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

        Ok(SheetData { sheet, columns, rows, row_count })
    }

    pub fn create_sheet(&self, workbook_id: i64, name: &str) -> Result<Sheet> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        conn.execute("UPDATE workbooks SET updated_at = datetime('now') WHERE id = ?1", params![workbook_id])?;
        Ok(Sheet { id, workbook_id, name: name.to_string(), sort_order: max_order + 1, created_at: chrono::Utc::now().to_rfc3339() })
    }

    pub fn add_column(&self, sheet_id: i64, name: &str, col_type: &str) -> Result<SheetColumn> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        Ok(SheetColumn { id, sheet_id, name: name.to_string(), col_type: col_type.to_string(), sort_order: max_order + 1, width: 150 })
    }

    pub fn update_cell(&self, sheet_id: i64, row_index: i64, column_id: i64, value: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT INTO cell_data (sheet_id, row_index, column_id, value) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(sheet_id, row_index, column_id) DO UPDATE SET value = ?4",
            params![sheet_id, row_index, column_id, value],
        )?;
        Ok(())
    }

    pub fn add_row(&self, sheet_id: i64) -> Result<i64> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let max_row: i64 = conn.query_row(
            "SELECT COALESCE(MAX(row_index), -1) FROM cell_data WHERE sheet_id = ?1",
            params![sheet_id],
            |row| row.get(0),
        )?;
        let new_row = max_row + 1;
        let mut col_stmt = conn.prepare("SELECT id, col_type FROM sheet_columns WHERE sheet_id = ?1 ORDER BY sort_order")?;
        let cols: Vec<(i64, String)> = col_stmt.query_map(params![sheet_id], |row| {
            Ok((row.get(0)?, row.get::<_, String>(1)?))
        })?.collect::<Result<Vec<_>>>()?;
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("UPDATE workbooks SET name = ?1, updated_at = datetime('now') WHERE id = ?2", params![name, id])?;
        Ok(())
    }

    pub fn replace_sheet_data(&self, sheet_id: i64, columns: &[(&str, &str)], rows_data: &[Vec<&str>]) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute("DELETE FROM cell_data WHERE sheet_id = ?1", params![sheet_id])?;
        conn.execute("DELETE FROM sheet_columns WHERE sheet_id = ?1", params![sheet_id])?;
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT business_name, industry, company_size, completed_onboarding FROM business_info WHERE id = 1"
        )?;
        let result = stmt.query_row([], |row| {
            Ok(BusinessInfo {
                business_name: row.get(0)?,
                industry: row.get(1)?,
                company_size: row.get(2)?,
                completed_onboarding: row.get::<_, i32>(3)? != 0,
            })
        });
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn save_business_info(&self, business_name: &str, industry: &str, company_size: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "INSERT OR REPLACE INTO business_info (id, business_name, industry, company_size, completed_onboarding, updated_at)
             VALUES (1, ?1, ?2, ?3, 1, datetime('now'))",
            params![business_name, industry, company_size],
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
        if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check()", [], |row| row.get::<_, i64>(0)) {
            if count > 0 {
                result.is_valid = false;
                result.errors.push(format!("Found {} foreign key constraint violations", count));
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
                result.errors.push(format!("Found {} orphaned sheet_columns records", count));
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
                result.errors.push(format!("Found {} cell_data records with invalid sheet_id", count));
            }
        }

        if let Ok(count) = conn.query_row(
            "SELECT COUNT(*) FROM cell_data WHERE column_id NOT IN (SELECT id FROM sheet_columns)",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            if count > 0 {
                result.is_valid = false;
                result.errors.push(format!("Found {} cell_data records with invalid column_id", count));
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
                result.errors.push(format!("Found {} orphaned sheets records", count));
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
                    result.errors.push(format!("Required table '{}' does not exist", table_name));
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
                    result.warnings.push(format!("Expected index '{}' does not exist", index_name));
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
}
