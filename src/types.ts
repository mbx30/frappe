export interface Workbook {
  id: number
  name: string
  created_at: string
  updated_at: string
}

export interface Sheet {
  id: number
  workbook_id: number
  name: string
  sort_order: number
  created_at: string
}

export interface SheetColumn {
  id: number
  sheet_id: number
  name: string
  col_type: string
  sort_order: number
  width: number
}

export interface CellData {
  id: number
  sheet_id: number
  row_index: number
  column_id: number
  value: string
}

export interface SheetData {
  sheet: Sheet
  columns: SheetColumn[]
  rows: CellData[][]
  row_count: number
}

export interface WorkbookData {
  workbook: Workbook
  sheets: SheetData[]
}

export interface ImportResult {
  rows_imported: number
  columns: string[]
  sheet_name: string
}

export interface GridRow {
  __row_index: number
  [key: string]: unknown
}

export interface BusinessInfo {
  business_name: string | null
  industry: string | null
  company_size: string | null
  order_number_prefix: string | null
  completed_onboarding: boolean
}

export interface InvoiceLineItem {
  id: number
  invoice_id: number
  description: string
  quantity: number
  unit_price: number
  sort_order: number
}

export interface Invoice {
  id: number
  invoice_number: string
  client_id: number | null
  status: 'draft' | 'sent' | 'paid' | 'overdue' | 'voided' | 'partially-paid'
  issue_date: string
  due_date: string
  payment_terms: string
  subtotal: number
  tax_rate: number
  tax_amount: number
  total: number
  currency: string
  internal_notes: string
  customer_notes: string
  qb_sync_status: 'not_synced' | 'synced' | 'pending' | 'error' | 'sync_error'
  amount_paid: number
  created_at: string
  updated_at: string
}

export interface InvoiceData {
  invoice: Invoice
  line_items: InvoiceLineItem[]
}

export interface Order {
  id: number
  order_number: string
  client_id: number | null
  status: 'prepress' | 'production' | 'delivery' | 'completed'
  priority: 'low' | 'normal' | 'high' | 'urgent'
  due_date: string
  description: string
  artwork_notes: string
  artwork_url: string | null
  artwork_approved: boolean
  deposit_requested: boolean
  deposit_amount: number
  total_value: number
  print_type: string
  paper_stock: string
  ink_colors: string
  finishing: string
  quantity: number
  production_notes: string
  assigned_operator: string
  fulfillment_method: 'pickup' | 'ship' | 'delivery'
  tracking_number: string
  tracking_carrier: string
  ready_for_pickup: boolean
  shipped_at: string | null
  created_at: string
  updated_at: string
}

export interface Payment {
  id: number
  invoice_id: number | null
  order_id: number | null
  amount: number
  payment_method: 'cash' | 'check' | 'card' | 'bank_transfer' | 'other'
  reference: string
  notes: string
  recorded_at: string
}

export interface InvoiceReminder {
  id: number
  invoice_id: number
  method: 'email' | 'sms' | 'phone' | 'manual'
  notes: string
  created_at: string
}

export interface DepartmentNote {
  id: number
  order_id: number
  note: string
  department: 'design' | 'prepress' | 'press' | 'finishing' | 'shipping' | 'general'
  created_at: string
}

export interface OrderStatusHistory {
  id: number
  order_id: number
  previous_status: string
  new_status: string
  notes: string
  created_at: string
}

export interface OrderData {
  order: Order
  status_history: OrderStatusHistory[]
}

export interface EstimateLineItem {
  id: number
  estimate_id: number
  description: string
  category: 'labor' | 'materials' | 'inventory' | 'finishing'
  quantity: number
  unit_price: number
  sort_order: number
}

export interface Estimate {
  id: number
  estimate_number: string
  client_id: number | null
  status: 'draft' | 'sent' | 'approved' | 'rejected' | 'converted'
  valid_until: string
  subtotal: number
  tax_rate: number
  tax_amount: number
  total: number
  currency: string
  notes: string
  artwork_requirements: string
  converted_order_id: number | null
  created_at: string
  updated_at: string
}

export interface EstimateData {
  estimate: Estimate
  line_items: EstimateLineItem[]
}

export interface InventoryItem {
  id: number
  material_type: string
  size: string
  attributes: string
  quantity: number
  unit: string
  reorder_level: number
  alert_type: 'quantity' | 'percentage'
  alert_threshold: number
  last_restocked: string | null
  created_at: string
  updated_at: string
}

export interface InventoryTransaction {
  id: number
  inventory_item_id: number
  transaction_type: 'add' | 'remove' | 'adjust' | 'import'
  quantity_change: number
  reason: string
  related_order_id: number | null
  created_at: string
}

export interface InventoryAlert {
  id: number
  inventory_item_id: number
  alert_type: 'low_stock' | 'restock_needed'
  current_quantity: number
  threshold: number
  is_acknowledged: boolean
  created_at: string
}

export interface Client {
  id: number
  name: string
  company: string
  email: string
  phone: string
  address: string
  tags: string
  status: 'active' | 'inactive'
  notes: string
  last_contacted: string | null
  created_at: string
  updated_at: string
}

export interface ArtApproval {
  id: number
  order_id: number
  version: number
  file_path: string
  status: 'pending' | 'approved' | 'changes_requested'
  customer_notes: string
  staff_notes: string
  secure_token: string
  follow_up_hours: number
  follow_up_count: number
  submitted_at: string
  responded_at: string | null
  created_at: string
}

export interface PdfSummary {
  id: number
  file_path: string
  file_name: string
  page_count: number
  pdf_version: string
  file_size_bytes: number
  title: string
  creator: string
  producer: string
  creation_date: string
  is_encrypted: boolean
}

// ── Redaction (#231) ──────────────────────────────────────────────────────
// Coordinate contract: x/y/width/height are PDF points with a TOP-LEFT origin
// (y grows downward from the top of the page). The backend reads the page
// MediaBox and flips to PDF user space. `page` is the 0-based page index.
export interface RedactionRect {
  page: number
  x: number
  y: number
  width: number
  height: number
}

export interface RedactionResult {
  output_path: string
  pages_modified: number
  redactions_applied: number
  content_hash: string
}

export interface RedactionAuditEntry {
  id: number
  source_path: string
  output_path: string
  operator_name: string
  redaction_count: number
  pages_modified: number
  regions_json: string
  content_hash: string
  previous_hash: string | null
  notes: string
  created_at: string
  chain_valid: boolean
}

export interface FontFinding {
  font_name: string
  font_type: string
  is_embedded: boolean
  is_subsetted: boolean
  pages: number[]
  severity: string
  message: string
}

export interface PageBoxFinding {
  page: number
  box_type: string
  x: number
  y: number
  w: number
  h: number
  severity: string
  message: string
}

export interface ImageResolutionFinding {
  page: number
  image_name: string
  pixel_width: number
  pixel_height: number
  rendered_width_pts: number
  rendered_height_pts: number
  effective_dpi: number
  color_space: string
  severity: string
  message: string
}

export interface BleedFinding {
  page: number
  has_bleed_box: boolean
  bleed_top_mm: number
  bleed_right_mm: number
  bleed_bottom_mm: number
  bleed_left_mm: number
  min_required_mm: number
  severity: string
  message: string
}

export interface OutputIntent {
  s_key: string
  output_condition: string
  output_condition_id: string
  registry_name: string
  has_embedded_icc: boolean
  icc_num_channels: number
}

export interface SecurityFinding {
  category: string
  detail: string
  severity: string
  message: string
}

export interface PdfXFinding {
  category: string
  detail: string
  severity: string
  message: string
  fix_hint: string
}

export interface ColorSpaceFinding {
  color_space: string
  kind: string
  pages: number[]
  is_pdf_x_violation: boolean
  severity: string
  message: string
}

export interface OverprintFinding {
  page: number
  object_context: string
  overprint_stroke: boolean
  overprint_fill: boolean
  mode: string
  severity: string
  message: string
}

export interface TransparencyFinding {
  page: number
  ty: string
  value: string
  is_pdfx1a_violation: boolean
  severity: string
  message: string
}

export interface HiddenContentFinding {
  page: number
  ty: string
  description: string
  severity: string
}

export interface IccProfileInfo {
  name: string
  description: string
  color_space_type: string
  num_channels: number
  file_name: string
}

export interface SpotColorFinding {
  name: string
  pages: number[]
  has_alternate_colorspace: boolean
  alternate_colorspace_type: string
  severity: string
  message: string
}

export interface InkCoverageFinding {
  page: number
  max_tac: number
  average_tac: number
  exceeds_threshold: boolean
  severity: string
  message: string
}

export interface CombinedPreflightResult {
  fonts: FontFinding[]
  page_boxes: PageBoxFinding[]
  images: ImageResolutionFinding[]
  bleed: BleedFinding[]
  output_intents: OutputIntent[]
  security: SecurityFinding[]
  pdfx: PdfXFinding[]
  color_spaces: ColorSpaceFinding[]
  overprint: OverprintFinding[]
  transparency: TransparencyFinding[]
  hidden_content: HiddenContentFinding[]
}

export interface PreflightFinding {
  id: number
  job_id: number
  check_name: string
  severity: string
  page_num: number | null
  object_ref: string | null
  message: string
  fix_hint: string
  created_at: string
}

export interface CertifiedVersion {
  id: number
  job_id: number
  version_number: number
  file_path: string
  file_size_bytes: number
  author: string
  comment: string
  created_at: string
  is_signed: boolean
}

export interface ConversionResult {
  images_converted: number
  vector_ops_converted: number
  warnings: string[]
}

export interface PreflightRunSummary {
  id: number
  job_id: number
  profile: string
  total_errors: number
  total_warnings: number
  total_ok: number
  ran_at: string
}

// ── Phase 3 types ──────────────────────────────────────────────────────

export interface LayerInfo {
  name: string
  visible: boolean
  locked: boolean
  object_id: number
}

export interface TextMatch {
  page_index: number
  text: string
  char_index: number
  length: number
  bbox: [number, number, number, number] | null
}

export interface ReplaceResult {
  replacements_made: number
  output_path: string
}

export interface ImageInfo {
  name: string
  width: number
  height: number
  color_space: string
  bits_per_component: number
}

export interface OptimizeSettings {
  max_width?: number
  max_height?: number
  quality?: number
  convert_to_jpeg?: boolean
}

export interface PageDimensions {
  width_pts: number
  height_pts: number
  width_mm: number
  height_mm: number
}

// ── Phase 4 types ──────────────────────────────────────────────────────

export interface PreflightProfile {
  id: number
  name: string
  description: string
  is_builtin: boolean
  created_at: string
  updated_at: string
}

export interface ProfileCheck {
  id: number
  profile_id: number
  check_name: string
  severity: string
  enabled: boolean
  params: string
}

export interface ProfileFixup {
  id: number
  profile_id: number
  fixup_name: string
  enabled: boolean
  params: string
}

export interface ActionList {
  id: number
  name: string
  description: string
  created_at: string
  updated_at: string
}

export interface ActionListStep {
  id: number
  action_list_id: number
  step_order: number
  action_type: string
  params: string
}

export interface BatchJob {
  id: number
  action_list_id: number
  status: string
  total_files: number
  processed_files: number
  error_count: number
  started_at: string | null
  completed_at: string | null
  created_at: string
}

export interface BatchResult {
  id: number
  batch_id: number
  file_path: string
  status: string
  output_path: string | null
  error_message: string | null
  started_at: string | null
  completed_at: string | null
}

export interface HotFolder {
  id: number
  name: string
  watch_path: string
  action_list_id: number
  output_path: string
  file_pattern: string
  is_active: boolean
  created_at: string
  updated_at: string
}

// ── Phase 5 types ──────────────────────────────────────────────────────

export interface BarcodeResult {
  text: string
  format: string
  page: number
  bbox: [number, number, number, number] | null
}

export interface AnalyticsSummary {
  total_jobs: number
  total_preflight_runs: number
  total_errors: number
  total_warnings: number
  most_common_errors: [string, number][]
  jobs_by_day: [string, number][]
}

export interface ApprovalSheetConfig {
  title: string
  job_info: string
  pages: number[]
  notes: string
}

// ── Issue #230 PDF Annotation types ──────────────────────────────────────

export type AnnotationType = 'highlight' | 'underline' | 'strikethrough' | 'note'

export interface PdfAnnotation {
  id: number
  file_path: string
  page: number
  annotation_type: AnnotationType
  x: number
  y: number
  width: number
  height: number
  color: string
  content: string
  created_at: string
  updated_at: string
}

export interface PdfAnnotationReply {
  id: number
  annotation_id: number
  content: string
  created_at: string
}

// ── Issue #229 OCR types ──────────────────────────────────────────────

export type PdfType = 'TextBased' | 'Scanned' | {
  Mixed: {
    text_pages: number[]
    scanned_pages: number[]
  }
}

export type OcrBackend = 'Tesseract' | 'GoogleCloudVision'

export interface OcrTextRegion {
  text: string
  bbox: [number, number, number, number]  // [x, y, width, height]
  confidence: number
}

export interface OcrPageResult {
  page_index: number
  text: string
  confidence: number
  regions: OcrTextRegion[]
}

export interface OcrOptions {
  pages: number[]
  backend: OcrBackend
  overlay_text: boolean
  output_path: string | null
  language: string
}

export interface OcrResult {
  pages: OcrPageResult[]
  total_text: string
  backend: string
  pages_processed: number
  duration_ms: number
}

export interface CostEstimate {
  page_count: number
  billable_pages: number
  cost_usd: number
  currency: string
}

// ── Phase 6 types ──────────────────────────────────────────────────────

export interface EmailSettings {
  smtp_host: string
  smtp_port: number
  smtp_username: string
  smtp_password: string
  use_tls: boolean
  from_address: string
}

export interface FtpSettings {
  host: string
  port: number
  username: string
  password: string
  remote_dir: string
}

export interface WebhookEntry {
  id: number
  url: string
  event: string
  is_active: boolean
  created_at: string
}

// ── Invoice status machine ────────────────────────────────────────────
export type InvoiceStatus = 'draft' | 'sent' | 'paid' | 'overdue' | 'voided' | 'partially-paid'

const INVOICE_TRANSITIONS: Record<InvoiceStatus, InvoiceStatus[]> = {
  draft: ['sent', 'voided'],
  sent: ['partially-paid', 'paid', 'overdue', 'voided'],
  'partially-paid': ['paid', 'overdue', 'voided'],
  paid: ['voided'],
  overdue: ['partially-paid', 'paid', 'voided'],
  voided: [],
}

export function allowedInvoiceTransitions(current: InvoiceStatus): InvoiceStatus[] {
  return INVOICE_TRANSITIONS[current] ?? []
}

export function isValidInvoiceTransition(current: InvoiceStatus, next: InvoiceStatus): boolean {
  return INVOICE_TRANSITIONS[current]?.includes(next) ?? false
}

export function invoiceStatusLabel(status: InvoiceStatus): string {
  switch (status) {
    case 'draft': return 'Draft'
    case 'sent': return 'Sent'
    case 'partially-paid': return 'Partially Paid'
    case 'paid': return 'Paid'
    case 'overdue': return 'Overdue'
    case 'voided': return 'Voided'
  }
}

// ── Estimate status machine ───────────────────────────────────────────
export type EstimateStatus = 'draft' | 'sent' | 'approved' | 'rejected' | 'converted'

const ESTIMATE_TRANSITIONS: Record<EstimateStatus, EstimateStatus[]> = {
  draft: ['sent'],
  sent: ['approved', 'rejected', 'draft'],
  approved: ['converted', 'rejected'],
  rejected: ['draft'],
  converted: [],
}

export function allowedEstimateTransitions(current: EstimateStatus): EstimateStatus[] {
  return ESTIMATE_TRANSITIONS[current] ?? []
}

export function isValidEstimateTransition(current: EstimateStatus, next: EstimateStatus): boolean {
  return ESTIMATE_TRANSITIONS[current]?.includes(next) ?? false
}

export function estimateStatusLabel(status: EstimateStatus): string {
  switch (status) {
    case 'draft': return 'Draft'
    case 'sent': return 'Sent'
    case 'approved': return 'Approved'
    case 'rejected': return 'Rejected'
    case 'converted': return 'Converted to Order'
  }
}
