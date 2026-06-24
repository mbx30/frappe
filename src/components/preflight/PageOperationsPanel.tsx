import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { t } from '../../i18n'

interface PageOperationsPanelProps {
  filePath?: string
  pageCount?: number
}

function deriveOutPath(filePath: string, suffix: string): string {
  return filePath.replace(/\.pdf$/i, `_${suffix}.pdf`)
}

export default function PageOperationsPanel({ filePath, pageCount }: PageOperationsPanelProps) {
  const [busy, setBusy] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [rotateDegrees, setRotateDegrees] = useState(90)
  const [pageIndex, setPageIndex] = useState(0)
  const [indices, setIndices] = useState('')
  const [insertAfter, setInsertAfter] = useState(0)
  const [widthMm, setWidthMm] = useState(210)
  const [heightMm, setHeightMm] = useState(297)
  const [newOrder, setNewOrder] = useState('')

  const runOp = useCallback(
    async (op: string, args: Record<string, unknown>) => {
      if (!filePath) return
      setBusy(op)
      setError(null)
      setMessage(null)
      try {
        const out = deriveOutPath(filePath, op)
        await invoke(op, { ...args, path: filePath, outputPath: out })
        setMessage(`${op} ok -> ${out}`)
      } catch (e) {
        setError(String(e))
      } finally {
        setBusy(null)
      }
    },
    [filePath]
  )

  const parseIndices = (raw: string): number[] =>
    raw
      .split(/[,\s]+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0)
      .map((s) => Number(s))
      .filter((n) => Number.isFinite(n) && n >= 0)

  if (!filePath) {
    return (
      <div className="page-operations-panel">
        <h3>{t('pageops.title')}</h3>
        <p className="pdf-empty">Open a PDF to perform page operations.</p>
      </div>
    )
  }

  return (
    <div className="page-operations-panel">
      <h3>{t('pageops.title')}</h3>
      {pageCount && <p className="page-operations-count">{pageCount} pages</p>}
      {error && <p className="pdf-error">{error}</p>}
      {message && <p className="page-operations-ok">{message}</p>}

      <section className="page-op">
        <h4>Extract pages</h4>
        <input
          type="text"
          placeholder="e.g. 0,2,4"
          value={indices}
          onChange={(e) => setIndices(e.target.value)}
        />
        <button
          className="btn btn-secondary"
          disabled={busy !== null}
          onClick={() =>
            runOp('extract_pages', { indices: parseIndices(indices) })
          }
        >
          Extract
        </button>
      </section>

      <section className="page-op">
        <h4>Delete pages</h4>
        <input
          type="text"
          placeholder="e.g. 0,2,4"
          value={indices}
          onChange={(e) => setIndices(e.target.value)}
        />
        <button
          className="btn btn-secondary"
          disabled={busy !== null}
          onClick={() =>
            runOp('delete_pages', { indices: parseIndices(indices) })
          }
        >
          Delete
        </button>
      </section>

      <section className="page-op">
        <h4>Rotate page</h4>
        <input
          type="number"
          min={0}
          max={pageCount ? pageCount - 1 : 0}
          value={pageIndex}
          onChange={(e) => setPageIndex(Number(e.target.value))}
        />
        <select
          value={rotateDegrees}
          onChange={(e) => setRotateDegrees(Number(e.target.value))}
        >
          <option value={90}>90°</option>
          <option value={180}>180°</option>
          <option value={270}>270°</option>
        </select>
        <button
          className="btn btn-secondary"
          disabled={busy !== null}
          onClick={() =>
            runOp('rotate_page', { pageIndex, degrees: rotateDegrees })
          }
        >
          Rotate
        </button>
      </section>

      <section className="page-op">
        <h4>Reorder pages</h4>
        <input
          type="text"
          placeholder="e.g. 2,0,1"
          value={newOrder}
          onChange={(e) => setNewOrder(e.target.value)}
        />
        <button
          className="btn btn-secondary"
          disabled={busy !== null}
          onClick={() =>
            runOp('reorder_pages', { newOrder: parseIndices(newOrder) })
          }
        >
          Reorder
        </button>
      </section>

      <section className="page-op">
        <h4>Insert blank page</h4>
        <input
          type="number"
          placeholder="after index"
          value={insertAfter}
          onChange={(e) => setInsertAfter(Number(e.target.value))}
        />
        <input
          type="number"
          placeholder="width mm"
          value={widthMm}
          onChange={(e) => setWidthMm(Number(e.target.value))}
        />
        <input
          type="number"
          placeholder="height mm"
          value={heightMm}
          onChange={(e) => setHeightMm(Number(e.target.value))}
        />
        <button
          className="btn btn-secondary"
          disabled={busy !== null}
          onClick={() =>
            runOp('insert_blank_page', {
              afterIndex: insertAfter,
              widthMm,
              heightMm,
            })
          }
        >
          Insert
        </button>
      </section>
    </div>
  )
}
