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
