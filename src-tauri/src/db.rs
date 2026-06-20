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
            CREATE INDEX IF NOT EXISTS idx_alerts_item ON inventory_alerts(inventory_item_id);"
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

    pub fn create_invoice(&self, invoice_number: &str, due_date: &str, payment_terms: &str) -> Result<Invoice> {
        if invoice_number.trim().is_empty() || due_date.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_invoices(&self) -> Result<Vec<Invoice>> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_number, client_id, status, issue_date, due_date, payment_terms,
                    subtotal, tax_rate, tax_amount, total, currency, internal_notes, customer_notes,
                    created_at, updated_at FROM invoices ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
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
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_invoice_data(&self, invoice_id: i64) -> Result<InvoiceData> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_number, client_id, status, issue_date, due_date, payment_terms,
                    subtotal, tax_rate, tax_amount, total, currency, internal_notes, customer_notes,
                    created_at, updated_at FROM invoices WHERE id = ?1"
        )?;
        let invoice = stmt.query_row(params![invoice_id], |row| {
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
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, invoice_id, description, quantity, unit_price, sort_order
             FROM invoice_line_items WHERE invoice_id = ?1 ORDER BY sort_order"
        )?;
        let line_items = stmt.query_map(params![invoice_id], |row| {
            Ok(InvoiceLineItem {
                id: row.get(0)?,
                invoice_id: row.get(1)?,
                description: row.get(2)?,
                quantity: row.get(3)?,
                unit_price: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(InvoiceData { invoice, line_items })
    }

    pub fn add_line_item(&self, invoice_id: i64, description: &str, quantity: f64, unit_price: f64) -> Result<InvoiceLineItem> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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

    pub fn update_invoice(&self, id: i64, status: &str, subtotal: f64, tax_rate: f64, tax_amount: f64, total: f64, internal_notes: &str, customer_notes: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE invoices SET status = ?1, subtotal = ?2, tax_rate = ?3, tax_amount = ?4, total = ?5, internal_notes = ?6, customer_notes = ?7, updated_at = datetime('now') WHERE id = ?8",
            params![status, subtotal, tax_rate, tax_amount, total, internal_notes, customer_notes, id],
        )?;
        Ok(())
    }

    pub fn create_order(&self, order_number: &str, due_date: &str, description: &str) -> Result<Order> {
        if order_number.trim().is_empty() || due_date.trim().is_empty() || description.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_orders(&self) -> Result<Vec<Order>> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, order_number, client_id, status, priority, due_date, description, artwork_notes, artwork_url,
                    artwork_approved, deposit_requested, deposit_amount, total_value, created_at, updated_at
             FROM orders ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
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
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_order_data(&self, order_id: i64) -> Result<OrderData> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, order_number, client_id, status, priority, due_date, description, artwork_notes, artwork_url,
                    artwork_approved, deposit_requested, deposit_amount, total_value, created_at, updated_at
             FROM orders WHERE id = ?1"
        )?;
        let order = stmt.query_row(params![order_id], |row| {
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
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, order_id, previous_status, new_status, notes, created_at
             FROM order_status_history WHERE order_id = ?1 ORDER BY created_at DESC"
        )?;
        let status_history = stmt.query_map(params![order_id], |row| {
            Ok(OrderStatusHistory {
                id: row.get(0)?,
                order_id: row.get(1)?,
                previous_status: row.get(2)?,
                new_status: row.get(3)?,
                notes: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(OrderData { order, status_history })
    }

    pub fn update_order_status(&self, order_id: i64, new_status: &str, notes: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;

        let previous_status: String = conn.query_row(
            "SELECT status FROM orders WHERE id = ?1",
            params![order_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO order_status_history (order_id, previous_status, new_status, notes)
             VALUES (?1, ?2, ?3, ?4)",
            params![order_id, previous_status, new_status, notes],
        )?;

        conn.execute(
            "UPDATE orders SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![new_status, order_id],
        )?;

        Ok(())
    }

    pub fn update_order(&self, id: i64, priority: &str, description: &str, artwork_notes: &str, artwork_approved: bool, deposit_requested: bool, deposit_amount: f64, total_value: f64) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE orders SET priority = ?1, description = ?2, artwork_notes = ?3, artwork_approved = ?4, deposit_requested = ?5, deposit_amount = ?6, total_value = ?7, updated_at = datetime('now') WHERE id = ?8",
            params![priority, description, artwork_notes, artwork_approved as i32, deposit_requested as i32, deposit_amount, total_value, id],
        )?;
        Ok(())
    }

    pub fn create_estimate(&self, estimate_number: &str, valid_until: &str) -> Result<Estimate> {
        if estimate_number.trim().is_empty() || valid_until.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, estimate_number, client_id, status, valid_until, subtotal, tax_rate, tax_amount, total, currency, notes, artwork_requirements, converted_order_id, created_at, updated_at FROM estimates ORDER BY created_at DESC"
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        let line_items = stmt.query_map(params![estimate_id], |row| {
            Ok(EstimateLineItem {
                id: row.get(0)?,
                estimate_id: row.get(1)?,
                description: row.get(2)?,
                category: row.get(3)?,
                quantity: row.get(4)?,
                unit_price: row.get(5)?,
                sort_order: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(EstimateData { estimate, line_items })
    }

    pub fn add_estimate_line_item(&self, estimate_id: i64, description: &str, category: &str, quantity: f64, unit_price: f64) -> Result<EstimateLineItem> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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

    pub fn update_estimate(&self, id: i64, status: &str, subtotal: f64, tax_rate: f64, tax_amount: f64, total: f64, notes: &str, artwork_requirements: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        conn.execute(
            "UPDATE estimates SET status = ?1, subtotal = ?2, tax_rate = ?3, tax_amount = ?4, total = ?5, notes = ?6, artwork_requirements = ?7, updated_at = datetime('now') WHERE id = ?8",
            params![status, subtotal, tax_rate, tax_amount, total, notes, artwork_requirements, id],
        )?;
        Ok(())
    }

    pub fn add_inventory_item(&self, material_type: &str, size: &str, attributes: &str, quantity: f64, unit: &str, reorder_level: f64, alert_type: &str, alert_threshold: f64) -> Result<InventoryItem> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, material_type, size, attributes, quantity, unit, reorder_level, alert_type, alert_threshold, last_restocked, created_at, updated_at FROM inventory_items ORDER BY material_type, size"
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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

    pub fn adjust_inventory(&self, inventory_item_id: i64, quantity_change: f64, reason: &str, order_id: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Guard: prevent quantity going below zero
        if quantity_change < 0.0 {
            let current: f64 = conn.query_row(
                "SELECT quantity FROM inventory_items WHERE id = ?1",
                params![inventory_item_id],
                |row| row.get(0),
            )?;
            if current + quantity_change < 0.0 {
                return Err(rusqlite::Error::InvalidQuery);
            }
        }

        conn.execute(
            "INSERT INTO inventory_transactions (inventory_item_id, transaction_type, quantity_change, reason, related_order_id) VALUES (?1, 'adjust', ?2, ?3, ?4)",
            params![inventory_item_id, quantity_change, reason, order_id],
        )?;

        conn.execute(
            "UPDATE inventory_items SET quantity = quantity + ?1, updated_at = datetime('now') WHERE id = ?2",
            params![quantity_change, inventory_item_id],
        )?;

        let current_qty: f64 = conn.query_row(
            "SELECT quantity FROM inventory_items WHERE id = ?1",
            params![inventory_item_id],
            |row| row.get(0),
        )?;

        let (alert_type, threshold): (String, f64) = conn.query_row(
            "SELECT alert_type, alert_threshold FROM inventory_items WHERE id = ?1",
            params![inventory_item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let should_alert = match alert_type.as_str() {
            "quantity" => current_qty <= threshold,
            "percentage" => {
                let reorder: f64 = conn.query_row(
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
            conn.execute(
                "INSERT OR IGNORE INTO inventory_alerts (inventory_item_id, alert_type, current_quantity, threshold, is_acknowledged) VALUES (?1, 'low_stock', ?2, ?3, 0)",
                params![inventory_item_id, current_qty, threshold],
            )?;
        }

        Ok(())
    }

    pub fn get_low_stock_alerts(&self) -> Result<Vec<InventoryAlert>> {
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut stmt = conn.prepare(
            "SELECT id, inventory_item_id, alert_type, current_quantity, threshold, is_acknowledged, created_at FROM inventory_alerts WHERE is_acknowledged = 0 ORDER BY created_at DESC"
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
        let conn = self.conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
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
