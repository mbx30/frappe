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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceLineItem {
    pub id: i64,
    pub invoice_id: i64,
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invoice {
    pub id: i64,
    pub invoice_number: String,
    pub client_id: Option<i64>,
    pub status: String, // draft, sent, paid, overdue, voided
    pub issue_date: String,
    pub due_date: String,
    pub payment_terms: String, // net-15, net-30, custom
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub currency: String,
    pub internal_notes: String,
    pub customer_notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceData {
    pub invoice: Invoice,
    pub line_items: Vec<InvoiceLineItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: i64,
    pub order_number: String,
    pub client_id: Option<i64>,
    pub status: String, // prepress, production, delivery, completed
    pub priority: String, // low, normal, high, urgent
    pub due_date: String,
    pub description: String,
    pub artwork_notes: String,
    pub artwork_url: Option<String>,
    pub artwork_approved: bool,
    pub deposit_requested: bool,
    pub deposit_amount: f64,
    pub total_value: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderStatusHistory {
    pub id: i64,
    pub order_id: i64,
    pub previous_status: String,
    pub new_status: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderData {
    pub order: Order,
    pub status_history: Vec<OrderStatusHistory>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EstimateLineItem {
    pub id: i64,
    pub estimate_id: i64,
    pub description: String,
    pub category: String, // labor, materials, inventory, finishing
    pub quantity: f64,
    pub unit_price: f64,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Estimate {
    pub id: i64,
    pub estimate_number: String,
    pub client_id: Option<i64>,
    pub status: String, // draft, sent, approved, rejected, converted
    pub valid_until: String,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub currency: String,
    pub notes: String,
    pub artwork_requirements: String,
    pub converted_order_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EstimateData {
    pub estimate: Estimate,
    pub line_items: Vec<EstimateLineItem>,
}
