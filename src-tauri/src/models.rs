use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workbook {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sheet {
    pub id: i64,
    pub workbook_id: i64,
    pub name: String,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SheetColumn {
    pub id: i64,
    pub sheet_id: i64,
    pub name: String,
    pub col_type: String,
    pub sort_order: i64,
    pub width: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CellData {
    pub id: i64,
    pub sheet_id: i64,
    pub row_index: i64,
    pub column_id: i64,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SheetData {
    pub sheet: Sheet,
    pub columns: Vec<SheetColumn>,
    pub rows: Vec<Vec<CellData>>,
    pub row_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkbookData {
    pub workbook: Workbook,
    pub sheets: Vec<SheetData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub rows_imported: usize,
    pub columns: Vec<String>,
    pub sheet_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BusinessInfo {
    pub business_name: Option<String>,
    pub industry: Option<String>,
    pub company_size: Option<String>,
    pub completed_onboarding: bool,
}
