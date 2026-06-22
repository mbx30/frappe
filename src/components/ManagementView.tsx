import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { Workbook, WorkbookData, SheetData, Client, PdfSummary } from '../types'
import WorkbookList from './WorkbookList'
import Spreadsheet from './Spreadsheet'
import Toolbar from './Toolbar'
import CloudImportDialog from './CloudImportDialog'
import Dashboard from './Dashboard'
import OrderList from './OrderList'
import OrderDetail from './OrderDetail'
import InvoiceList from './InvoiceList'
import InvoiceEditor from './InvoiceEditor'
import EstimateList from './EstimateList'
import EstimateEditor from './EstimateEditor'
import InventoryList from './InventoryList'
import ClientList from './ClientList'
import ClientForm from './ClientForm'
import POSView from './POSView'
import QBSyncPanel from './QBSyncPanel'
import PDFView from './PDFView'
import './ManagementView.css'

type Section =
  | 'dashboard'
  | 'workbooks'
  | 'orders'
  | 'invoices'
  | 'estimates'
  | 'inventory'
  | 'clients'
  | 'pos'
  | 'qb'
  | 'pdf'

const NAV_ITEMS: { id: Section; label: string; icon: string }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: '⊞' },
  { id: 'workbooks', label: 'Workbooks', icon: '📋' },
  { id: 'orders', label: 'Orders', icon: '📦' },
  { id: 'estimates', label: 'Estimates', icon: '📝' },
  { id: 'invoices', label: 'Invoices', icon: '🧾' },
  { id: 'inventory', label: 'Inventory', icon: '🗄' },
  { id: 'clients', label: 'Clients', icon: '👥' },
  { id: 'pos', label: 'Point of Sale', icon: '$' },
  { id: 'qb', label: 'QuickBooks', icon: '⚡' },
  { id: 'pdf', label: 'PDF Tools', icon: '📄' },
]

export default function ManagementView() {
  const [section, setSection] = useState<Section>('dashboard')

  // Workbook state
  const [workbooks, setWorkbooks] = useState<Workbook[]>([])
  const [activeId, setActiveId] = useState<number | null>(null)
  const [activeWorkbook, setActiveWorkbook] = useState<WorkbookData | null>(null)
  const [activeSheetIdx, setActiveSheetIdx] = useState(0)
  const [showCloudImport, setShowCloudImport] = useState(false)

  // Order state
  const [editingOrderId, setEditingOrderId] = useState<number | null | undefined>(undefined)

  // Invoice state
  const [editingInvoiceId, setEditingInvoiceId] = useState<number | null | undefined>(undefined)

  // Estimate state
  const [editingEstimateId, setEditingEstimateId] = useState<number | null | undefined>(undefined)

  // Client state
  const [editingClient, setEditingClient] = useState<Client | null | undefined>(undefined)
  const [importError, setImportError] = useState<string | null>(null)

  // PDF state
  const [pdfSummary, setPdfSummary] = useState<PdfSummary | null>(null)
  const [pdfJobs, setPdfJobs] = useState<PdfSummary[]>([])

  const loadWorkbooks = useCallback(async () => {
    const list = await invoke<Workbook[]>('list_workbooks')
    setWorkbooks(list)
  }, [])

  // Generation token: bumped at the start of every save mutation AND at
  // the start of every loadWorkbook. Any reload whose `await` resolves
  // after the token has moved on is discarded. This prevents:
  //   1. Overlapping loadWorkbook calls (A-then-B race) — token bumps
  //      inside loadWorkbook, the slower response sees a newer token.
  //   2. Save-then-reload races — the save bumps the token first, so
  //      a reload that was already in flight from a previous save is
  //      invalidated and cannot overwrite the new state.
  const reqIdRef = useRef(0)
  const bumpGen = useCallback(() => {
    reqIdRef.current += 1
  }, [])
  const loadWorkbook = useCallback(async (id: number) => {
    const myId = ++reqIdRef.current
    try {
      const data = await invoke<WorkbookData>('get_workbook', { id })
      if (myId !== reqIdRef.current) return
      setActiveWorkbook(data)
      setActiveSheetIdx(0)
    } catch (e) {
      // Only re-throw if this is still the active request; otherwise the
      // error is from a stale call and should be swallowed so it cannot
      // surface a confusing "load failed" after a successful newer save.
      if (myId === reqIdRef.current) throw e
    }
  }, [])

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { loadWorkbooks() }, [loadWorkbooks])
  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { if (activeId) loadWorkbook(activeId) }, [activeId, loadWorkbook])

  const handleCreateWorkbook = async () => {
    // Use timestamp-based name to guarantee uniqueness across rapid
    // create/delete races where workbooks.length + 1 would collide.
    const name = `Workbook ${new Date().toISOString().slice(0, 16).replace('T', ' ')}`
    bumpGen()
    const wb = await invoke<Workbook>('create_workbook', { name })
    setWorkbooks((prev) => [...prev, wb])
    setActiveId(wb.id)
    setSection('workbooks')
  }

  const handleDeleteWorkbook = async (id: number) => {
    bumpGen()
    try {
      await invoke('delete_workbook', { id })
      if (activeId === id) { setActiveId(null); setActiveWorkbook(null) }
      await loadWorkbooks()
    } catch (e) {
      alert(`Failed to delete workbook: ${e}`)
    }
  }

  const handleRenameWorkbook = async () => {
    if (!activeWorkbook) return
    const name = prompt('Workbook name:', activeWorkbook.workbook.name)
    if (name && name !== activeWorkbook.workbook.name) {
      bumpGen()
      await invoke('update_workbook_name', { id: activeWorkbook.workbook.id, name })
      loadWorkbook(activeWorkbook.workbook.id)
    }
  }

  const importFile = async (format: 'csv' | 'excel') => {
    if (!activeWorkbook) return
    const extensions = format === 'csv'
      ? [{ name: 'CSV', extensions: ['csv'] }]
      : [{ name: 'Excel', extensions: ['xlsx', 'xls'] }]
    const filePath = await open({ filters: extensions, multiple: false })
    if (!filePath) return
    try {
      bumpGen()
      await invoke<SheetData>(format === 'csv' ? 'import_csv_file' : 'import_excel_file', {
        workbookId: activeWorkbook.workbook.id, filePath,
      })
      setImportError(null)
      loadWorkbook(activeWorkbook.workbook.id)
    } catch (e) {
      setImportError(`Import failed: ${e}`)
    }
  }

  const activeSheet: SheetData | null = activeWorkbook?.sheets[activeSheetIdx] ?? null

  // Stable callbacks for the Spreadsheet child. Without useCallback these
  // would be new function references on every render, which would defeat
  // the child's own useCallback-wrapped handlers and force them to be
  // re-invoked on each parent render. We re-create the callback whenever
  // activeWorkbook changes (which also re-derives activeSheet).
  const handleCellUpdate = useCallback(
    async (rowIndex: number, columnId: number, value: string) => {
      const sheet = activeWorkbook?.sheets[activeSheetIdx] ?? null
      if (!sheet) return
      bumpGen()
      await invoke('update_cell_value', { sheetId: sheet.sheet.id, rowIndex, columnId, value })
      if (activeWorkbook) loadWorkbook(activeWorkbook.workbook.id)
    },
    [activeWorkbook, activeSheetIdx, bumpGen, loadWorkbook]
  )

  const handleAddRow = useCallback(async () => {
    const sheet = activeWorkbook?.sheets[activeSheetIdx] ?? null
    if (!sheet) return
    bumpGen()
    await invoke('add_row', { sheetId: sheet.sheet.id })
    if (activeWorkbook) loadWorkbook(activeWorkbook.workbook.id)
  }, [activeWorkbook, activeSheetIdx, bumpGen, loadWorkbook])

  const renderSection = () => {
    switch (section) {
      case 'dashboard':
        return <Dashboard />

      case 'workbooks':
        return (
          <div className="workbooks-layout">
            <WorkbookList
              workbooks={workbooks}
              activeId={activeId}
              onSelect={(id) => { setActiveId(id); loadWorkbook(id) }}
              onCreate={handleCreateWorkbook}
              onDelete={handleDeleteWorkbook}
            />
            <div className="workbook-main">
              {importError && (
                <div className="import-error">{importError}</div>
              )}
              {activeWorkbook && activeSheet ? (
                <>
                  <Toolbar
                    workbookName={activeWorkbook.workbook.name}
                    onImportCsv={() => importFile('csv')}
                    onImportExcel={() => importFile('excel')}
                    onImportCloud={() => setShowCloudImport(true)}
                    onAddSheet={async () => {
                      if (!activeWorkbook) return
                      const name = prompt('Sheet name:', `Sheet ${activeWorkbook.sheets.length + 1}`)
                      if (name) {
                        bumpGen()
                        await invoke('create_sheet', { workbookId: activeWorkbook.workbook.id, name })
                        loadWorkbook(activeWorkbook.workbook.id)
                      }
                    }}
                    onRenameWorkbook={handleRenameWorkbook}
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
                <div className="workbook-empty">
                  <h2>Workbooks</h2>
                  <p>Create or select a workbook to get started.</p>
                  <button className="btn btn-primary" onClick={handleCreateWorkbook}>
                    Create Workbook
                  </button>
                </div>
              )}
            </div>
            {showCloudImport && activeWorkbook && (
              <CloudImportDialog
                workbookId={activeWorkbook.workbook.id}
                onClose={() => setShowCloudImport(false)}
                onImport={async (command, args) => {
                  bumpGen()
                  await invoke<SheetData>(command, args)
                  if (activeWorkbook) loadWorkbook(activeWorkbook.workbook.id)
                }}
              />
            )}
          </div>
        )

      case 'orders':
        if (editingOrderId !== undefined) {
          return (
            <OrderDetail
              orderId={editingOrderId ?? undefined}
              onSave={() => setEditingOrderId(undefined)}
              onCancel={() => setEditingOrderId(undefined)}
            />
          )
        }
        return (
          <OrderList
            onCreateNew={() => setEditingOrderId(null)}
            onSelectOrder={(id) => setEditingOrderId(id)}
          />
        )

      case 'invoices':
        if (editingInvoiceId !== undefined) {
          return (
            <InvoiceEditor
              invoiceId={editingInvoiceId ?? undefined}
              onSave={() => setEditingInvoiceId(undefined)}
              onCancel={() => setEditingInvoiceId(undefined)}
            />
          )
        }
        return (
          <InvoiceList
            onCreateNew={() => setEditingInvoiceId(null)}
            onEditInvoice={(id) => setEditingInvoiceId(id)}
          />
        )

      case 'estimates':
        if (editingEstimateId !== undefined) {
          return (
            <EstimateEditor
              estimateId={editingEstimateId ?? undefined}
              onSave={() => setEditingEstimateId(undefined)}
              onCancel={() => setEditingEstimateId(undefined)}
            />
          )
        }
        return (
          <EstimateList
            onCreateNew={() => setEditingEstimateId(null)}
            onSelectEstimate={(id) => setEditingEstimateId(id)}
          />
        )

      case 'inventory':
        return <InventoryList />

      case 'pos':
        return <POSView />

      case 'qb':
        return <QBSyncPanel />

      case 'pdf':
        return (
          <PDFView
            summary={pdfSummary}
            jobs={pdfJobs}
            onOpenFile={async () => {
              const filePath = await open({ filters: [{ name: 'PDF', extensions: ['pdf'] }], multiple: false })
              if (!filePath) return
              try {
                const summary = await invoke<PdfSummary>('open_pdf', { path: filePath })
                setPdfSummary(summary)
                const jobs = await invoke<PdfSummary[]>('list_pdf_jobs')
                setPdfJobs(jobs)
              } catch (e) {
                setImportError(`Failed to open PDF: ${e}`)
              }
            }}
            onSaveJob={async () => {
              if (!pdfSummary) return
              await invoke('save_pdf_job', { summary: pdfSummary })
              setPdfJobs(await invoke<PdfSummary[]>('list_pdf_jobs'))
            }}
            onDeleteJob={async (id: number) => {
              await invoke('delete_pdf_job', { id })
              setPdfJobs(await invoke<PdfSummary[]>('list_pdf_jobs'))
            }}
            onLoadJob={async (id: number) => {
              const job = pdfJobs.find(j => j.id === id)
              if (!job) return
              try {
                const summary = await invoke<PdfSummary>('open_pdf', { path: job.file_path })
                setPdfSummary(summary)
              } catch (e) {
                setImportError(`Failed to re-open PDF: ${e}`)
              }
            }}
            error={importError}
            onClearError={() => setImportError(null)}
          />
        )

      case 'clients':
        if (editingClient !== undefined) {
          return (
            <ClientForm
              client={editingClient ?? undefined}
              onSave={() => setEditingClient(undefined)}
              onCancel={() => setEditingClient(undefined)}
            />
          )
        }
        return (
          <ClientList
            onSelectClient={(c) => setEditingClient(c)}
            onNewClient={() => setEditingClient(null)}
          />
        )

      default:
        return null
    }
  }

  return (
    <div className="management-layout">
      <nav className="management-sidebar">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            className={`nav-item ${section === item.id ? 'nav-item--active' : ''}`}
            onClick={() => {
              // Warn if user has an editor open (unsaved changes would be lost)
              const hasUnsavedEditor =
                (section === 'orders' && editingOrderId !== undefined) ||
                (section === 'invoices' && editingInvoiceId !== undefined) ||
                (section === 'estimates' && editingEstimateId !== undefined) ||
                (section === 'clients' && editingClient !== undefined)
              if (hasUnsavedEditor && item.id !== section) {
                if (!confirm('Leave without saving? Any unsaved changes will be lost.')) return
              }
              setSection(item.id)
              setEditingOrderId(undefined)
              setEditingInvoiceId(undefined)
              setEditingEstimateId(undefined)
              setEditingClient(undefined)
              setImportError(null)
            }}
          >
            <span className="nav-icon">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
          </button>
        ))}
      </nav>

      <main className="management-content">
        {renderSection()}
      </main>
    </div>
  )
}
