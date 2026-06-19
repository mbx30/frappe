import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { Workbook, WorkbookData, SheetData } from './types'
import WorkbookList from './components/WorkbookList'
import Spreadsheet from './components/Spreadsheet'
import Toolbar from './components/Toolbar'
import CloudImportDialog from './components/CloudImportDialog'
import './App.css'

function App() {
  const [workbooks, setWorkbooks] = useState<Workbook[]>([])
  const [activeId, setActiveId] = useState<number | null>(null)
  const [activeWorkbook, setActiveWorkbook] = useState<WorkbookData | null>(null)
  const [activeSheetIdx, setActiveSheetIdx] = useState(0)
  const [showCloudImport, setShowCloudImport] = useState(false)

  const loadWorkbooks = useCallback(async () => {
    const list = await invoke<Workbook[]>('list_workbooks')
    setWorkbooks(list)
  }, [])

  const loadWorkbook = useCallback(async (id: number) => {
    const data = await invoke<WorkbookData>('get_workbook', { id })
    setActiveWorkbook(data)
    setActiveSheetIdx(0)
  }, [])

  useEffect(() => { loadWorkbooks() }, [loadWorkbooks])

  useEffect(() => {
    if (activeId) loadWorkbook(activeId)
  }, [activeId, loadWorkbook])

  const handleCreate = async () => {
    const name = `Workbook ${workbooks.length + 1}`
    const wb = await invoke<Workbook>('create_workbook', { name })
    setWorkbooks((prev) => [...prev, wb])
    setActiveId(wb.id)
  }

  const handleDelete = async (id: number) => {
    await invoke('delete_workbook', { id })
    if (activeId === id) {
      setActiveId(null)
      setActiveWorkbook(null)
    }
    loadWorkbooks()
  }

  const handleRename = async () => {
    if (!activeWorkbook) return
    const name = prompt('Workbook name:', activeWorkbook.workbook.name)
    if (name && name !== activeWorkbook.workbook.name) {
      await invoke('update_workbook_name', { id: activeWorkbook.workbook.id, name })
      loadWorkbook(activeWorkbook.workbook.id)
    }
  }

  const activeSheet: SheetData | null = activeWorkbook?.sheets[activeSheetIdx] ?? null

  const handleCellUpdate = async (rowIndex: number, columnId: number, value: string) => {
    if (!activeSheet) return
    await invoke('update_cell_value', {
      sheetId: activeSheet.sheet.id,
      rowIndex,
      columnId,
      value,
    })
    if (activeWorkbook) loadWorkbook(activeWorkbook.workbook.id)
  }

  const handleAddRow = async () => {
    if (!activeSheet) return
    await invoke('add_row', { sheetId: activeSheet.sheet.id })
    if (activeWorkbook) loadWorkbook(activeWorkbook.workbook.id)
  }

  const handleAddSheet = async () => {
    if (!activeWorkbook) return
    const name = prompt('Sheet name:', `Sheet ${activeWorkbook.sheets.length + 1}`)
    if (name) {
      await invoke('create_sheet', { workbookId: activeWorkbook.workbook.id, name })
      loadWorkbook(activeWorkbook.workbook.id)
    }
  }

  const importFile = async (format: 'csv' | 'excel') => {
    if (!activeWorkbook) return
    const extensions = format === 'csv' ? [{ name: 'CSV', extensions: ['csv'] }] : [{ name: 'Excel', extensions: ['xlsx', 'xls'] }]
    const filePath = await open({ filters: extensions, multiple: false })
    if (!filePath) return

    const cmd = format === 'csv' ? 'import_csv_file' : 'import_excel_file'
    try {
      await invoke<SheetData>(cmd, { workbookId: activeWorkbook.workbook.id, filePath })
      loadWorkbook(activeWorkbook.workbook.id)
    } catch (e) {
      alert(`Import failed: ${e}`)
    }
  }

  const handleCloudImport = async (command: string, args: Record<string, unknown>) => {
    await invoke<SheetData>(command, args)
    if (activeWorkbook) loadWorkbook(activeWorkbook.workbook.id)
  }

  return (
    <div className="app-layout">
      <WorkbookList
        workbooks={workbooks}
        activeId={activeId}
        onSelect={(id) => { setActiveId(id); loadWorkbook(id) }}
        onCreate={handleCreate}
        onDelete={handleDelete}
      />
      <div className="main-area">
        {activeWorkbook && activeSheet ? (
          <>
            <Toolbar
              workbookName={activeWorkbook.workbook.name}
              onImportCsv={() => importFile('csv')}
              onImportExcel={() => importFile('excel')}
              onImportCloud={() => setShowCloudImport(true)}
              onAddSheet={handleAddSheet}
              onRenameWorkbook={handleRename}
            />
            <div className="sheet-tabs">
              {activeWorkbook.sheets.map((s, i) => (
                <button
                  key={s.sheet.id}
                  className={`sheet-tab ${i === activeSheetIdx ? 'active' : ''}`}
                  onClick={() => setActiveSheetIdx(i)}
                >
                  {s.sheet.name}
                </button>
              ))}
            </div>
            <div className="spreadsheet-wrapper">
              <Spreadsheet
                sheetData={activeSheet}
                onCellUpdate={handleCellUpdate}
                onAddRow={handleAddRow}
              />
            </div>
          </>
        ) : (
          <div className="empty-state">
            <h2>Frappe</h2>
            <p>Create a new workbook or select one to get started.</p>
            <button className="btn btn-primary" onClick={handleCreate}>Create Workbook</button>
          </div>
        )}
      </div>
      {showCloudImport && activeWorkbook && (
        <CloudImportDialog
          workbookId={activeWorkbook.workbook.id}
          onClose={() => setShowCloudImport(false)}
          onImport={handleCloudImport}
        />
      )}
    </div>
  )
}

export default App
