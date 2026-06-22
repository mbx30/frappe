import { useCallback, useMemo, useRef } from 'react'
import { DataGrid, type Column } from 'react-data-grid'
import 'react-data-grid/lib/styles.css'
import type { SheetData, GridRow } from '../types'

interface SpreadsheetProps {
  sheetData: SheetData
  onCellUpdate: (rowIndex: number, columnId: number, value: string) => void
  onAddRow: () => void
}

function rowKey(row: GridRow): number {
  return row.__row_index
}

export default function Spreadsheet({ sheetData, onCellUpdate, onAddRow }: SpreadsheetProps) {
  const pendingRef = useRef<Map<string, { rowIndex: number; columnId: number; value: string }>>(new Map())

  const columns: Column<GridRow>[] = useMemo(() => {
    return sheetData.columns.map((col) => ({
      key: `col_${col.id}`,
      name: col.name,
      width: col.width || 150,
      editable: true,
    }))
  }, [sheetData.columns])

  const rows: GridRow[] = useMemo(() => {
    const rowMap = new Map<number, GridRow>()
    for (const cellList of sheetData.rows) {
      for (const cell of cellList) {
        if (!rowMap.has(cell.row_index)) {
          rowMap.set(cell.row_index, { __row_index: cell.row_index })
        }
        const row = rowMap.get(cell.row_index)!
        row[`col_${cell.column_id}`] = cell.value
      }
    }
    return Array.from(rowMap.values()).sort((a, b) => a.__row_index - b.__row_index)
  }, [sheetData.rows])

  const onRowsChange = useCallback(
    (changedRows: GridRow[], data: { indexes: number[]; column: Column<GridRow> }) => {
      const colKey = data.column.key
      const colId = Number(colKey.replace('col_', ''))
      const pending = pendingRef.current

      for (const idx of data.indexes) {
        const row = changedRows[idx]
        if (!row) continue
        const key = `${row.__row_index}_${colId}`
        pending.set(key, {
          rowIndex: row.__row_index,
          columnId: colId,
          value: String(row[colKey] ?? ''),
        })
      }

      if (pending.size > 0) {
        for (const [, update] of pending) {
          onCellUpdate(update.rowIndex, update.columnId, update.value)
        }
        pending.clear()
      }
    },
    [onCellUpdate]
  )

  return (
    <div className="spreadsheet-container">
      <div className="spreadsheet-toolbar">
        <span className="sheet-name">{sheetData.sheet.name}</span>
        <button className="btn btn-sm" onClick={onAddRow}>+ Add Row</button>
      </div>
      <DataGrid
        columns={columns}
        rows={rows}
        rowKeyGetter={rowKey}
        onRowsChange={onRowsChange}
        defaultColumnOptions={{ resizable: true, sortable: true }}
        className="frappe-grid"
        direction="ltr"
      />
      <div className="spreadsheet-status">
        {rows.length} rows
      </div>
    </div>
  )
}
