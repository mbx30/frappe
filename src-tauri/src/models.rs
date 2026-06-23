use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginatedList<T: Serialize + Clone> {
    pub rows: Vec<T>,
    pub total_count: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerInfo {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub object_id: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextMatch {
    pub page_index: usize,
    pub text: String,
    pub char_index: usize,
    pub length: usize,
    pub bbox: Option<[f64; 4]>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReplaceResult {
    pub replacements_made: usize,
    pub output_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizeSettings {
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub quality: Option<u8>,
    pub convert_to_jpeg: Option<bool>,
}

// ── Preflight Profiles (Phase 4.1) ───────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreflightProfile {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreflightProfileInput {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileCheck {
    pub id: i64,
    pub profile_id: i64,
    pub check_name: String,
    pub severity: String,
    pub enabled: bool,
    pub params: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileFixup {
    pub id: i64,
    pub profile_id: i64,
    pub fixup_name: String,
    pub enabled: bool,
    pub params: String,
}

// ── Action Lists (Phase 4.2) ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionList {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionListInput {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionListStep {
    pub id: i64,
    pub action_list_id: i64,
    pub step_order: i64,
    pub action_type: String,
    pub params: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionListStepInput {
    pub action_type: String,
    pub params: String,
}

// ── Batch Processing (Phase 4.3) ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchJob {
    pub id: i64,
    pub action_list_id: i64,
    pub status: String,
    pub total_files: i64,
    pub processed_files: i64,
    pub error_count: i64,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchResult {
    pub id: i64,
    pub batch_id: i64,
    pub file_path: String,
    pub status: String,
    pub output_path: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

// ── Hot Folders (Phase 4.5) ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotFolder {
    pub id: i64,
    pub name: String,
    pub watch_path: String,
    pub action_list_id: i64,
    pub output_path: String,
    pub file_pattern: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotFolderInput {
    pub name: String,
    pub watch_path: String,
    pub action_list_id: i64,
    pub output_path: String,
    pub file_pattern: String,
}

// ── Barcode detection (Phase 5.2) ────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BarcodeResult {
    pub text: String,
    pub format: String,
    pub page: usize,
    pub bbox: Option<[f64; 4]>,
}

// ── Analytics (Phase 5.3) ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyticsSummary {
    pub total_jobs: i64,
    pub total_preflight_runs: i64,
    pub total_errors: i64,
    pub total_warnings: i64,
    pub most_common_errors: Vec<(String, i64)>,
    pub jobs_by_day: Vec<(String, i64)>,
}

// ── Approval sheet & export (Phase 5.4) ──────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApprovalSheetConfig {
    pub title: String,
    pub job_info: String,
    pub pages: Vec<usize>,
    pub notes: String,
}

// ── Email / FTP / Webhook (Phase 6.1) ───────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailSettings {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub use_tls: bool,
    pub from_address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FtpSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub remote_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookEntry {
    pub id: i64,
    pub url: String,
    pub event: String,
    pub is_active: bool,
    pub created_at: String,
}

// ── PreflightFinding (existing) ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreflightFinding {
    pub id: i64,
    #[serde(rename = "job_id")]
    pub run_id: i64,
    pub check_name: String,
    pub severity: String,
    pub page_num: Option<i64>,
    pub object_ref: Option<String>,
    pub message: String,
    pub fix_hint: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreflightFindingInput {
    pub check_name: String,
    pub severity: String,
    pub page_num: Option<i64>,
    pub object_ref: Option<String>,
    pub message: String,
    pub fix_hint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreflightRunSummary {
    pub id: i64,
    pub job_id: i64,
    pub profile: String,
    pub total_errors: i64,
    pub total_warnings: i64,
    pub total_ok: i64,
    pub ran_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CertifiedVersion {
    pub id: i64,
    pub job_id: i64,
    pub version_number: i32,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub author: String,
    pub comment: String,
    pub created_at: String,
    pub is_signed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PdfSummary {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub page_count: usize,
    pub pdf_version: String,
    pub file_size_bytes: u64,
    pub title: String,
    pub creator: String,
    pub producer: String,
    pub creation_date: String,
    pub is_encrypted: bool,
}

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
    pub order_number_prefix: Option<String>,
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
    pub status: String,
    pub issue_date: String,
    pub due_date: String,
    pub payment_terms: String,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub currency: String,
    pub internal_notes: String,
    pub customer_notes: String,
    pub qb_sync_status: String,
    pub amount_paid: f64,
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
    pub status: String,
    pub priority: String,
    pub due_date: String,
    pub description: String,
    pub artwork_notes: String,
    pub artwork_url: Option<String>,
    pub artwork_approved: bool,
    pub deposit_requested: bool,
    pub deposit_amount: f64,
    pub total_value: f64,
    pub print_type: String,
    pub paper_stock: String,
    pub ink_colors: String,
    pub finishing: String,
    pub quantity: i64,
    pub production_notes: String,
    pub assigned_operator: String,
    pub fulfillment_method: String,
    pub tracking_number: String,
    pub tracking_carrier: String,
    pub ready_for_pickup: bool,
    pub shipped_at: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryItem {
    pub id: i64,
    pub material_type: String,
    pub size: String,
    pub attributes: String, // JSON or comma-separated
    pub quantity: f64,
    pub unit: String, // pieces, sheets, kg, m, etc.
    pub reorder_level: f64,
    pub alert_type: String, // quantity or percentage
    pub alert_threshold: f64,
    pub last_restocked: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)] // Used by inventory_transactions table; exposed via future Tauri command
pub struct InventoryTransaction {
    pub id: i64,
    pub inventory_item_id: i64,
    pub transaction_type: String,
    pub quantity_change: f64,
    pub reason: String,
    pub related_order_id: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryAlert {
    pub id: i64,
    pub inventory_item_id: i64,
    pub alert_type: String, // low_stock, restock_needed
    pub current_quantity: f64,
    pub threshold: f64,
    pub is_acknowledged: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Client {
    pub id: i64,
    pub name: String,
    pub company: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub tags: String,   // comma-separated
    pub status: String, // active, inactive
    pub notes: String,
    pub last_contacted: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Payment {
    pub id: i64,
    pub invoice_id: Option<i64>,
    pub order_id: Option<i64>,
    pub amount: f64,
    pub payment_method: String, // cash, check, card, bank_transfer, other
    pub reference: String,      // check #, card last 4, etc.
    pub notes: String,
    pub recorded_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceReminder {
    pub id: i64,
    pub invoice_id: i64,
    pub method: String, // email, sms, phone, manual
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepartmentNote {
    pub id: i64,
    pub order_id: i64,
    pub note: String,
    pub department: String, // design, prepress, press, finishing, shipping, general
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtApproval {
    pub id: i64,
    pub order_id: i64,
    pub version: i64,
    pub file_path: String,
    pub status: String, // pending, approved, changes_requested
    pub customer_notes: String,
    pub staff_notes: String,
    pub secure_token: String,
    pub follow_up_hours: i64,
    pub follow_up_count: i64,
    pub submitted_at: String,
    pub responded_at: Option<String>,
    pub created_at: String,
}

// ── Schema versioning (#90) ─────────────────────────────────────────────

// ── Event log entry (#83) ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventLogEntry {
    pub id: i64,
    pub tenant_id: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: String,
    pub payload: String,
    pub created_at: String,
}

// ── Backup entry (#85) ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupEntry {
    pub id: i64,
    pub backup_type: String, // "snapshot" | "event_batch"
    pub file_path: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub created_at: String,
}
