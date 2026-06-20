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
  qb_sync_status: 'not_synced' | 'synced' | 'pending' | 'error'
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
