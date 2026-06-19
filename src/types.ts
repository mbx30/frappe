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
  created_at: string
  updated_at: string
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
