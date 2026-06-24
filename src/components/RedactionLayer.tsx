import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { RedactionRect, RedactionResult } from '../types'
import './RedactionLayer.css'

interface RedactionLayerProps {
  filePath: string
  pageCount: number
  initialPage: number
  onExported: (outputPath: string) => void
  onClose: () => void
}

// The redaction page is rendered at a fixed DPI so the pixel→point conversion
// is exact regardless of the main viewer's zoom. 1 pt = 1/72 inch.
const REDACT_DPI = 150

interface DraftRect {
  // Fractions (0..1) of the page image, top-left origin.
  x0: number
  y0: number
  x1: number
  y1: number
}

// A redaction is only meaningful above a tiny minimum size; this prevents an
// accidental click registering as a zero/near-zero-area redaction.
const MIN_FRACTION = 0.005

function deriveOutputPath(filePath: string): string {
  const dot = filePath.lastIndexOf('.')
  const slash = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'))
  if (dot > slash && dot > 0) {
    return filePath.slice(0, dot) + '.redacted.pdf'
  }
  return filePath + '.redacted.pdf'
}

export default function RedactionLayer({ filePath, pageCount, initialPage, onExported, onClose }: RedactionLayerProps) {
  const [pageIndex, setPageIndex] = useState(initialPage)
  const [renderUrl, setRenderUrl] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  // Page dimensions in PDF points, derived from the rendered image's natural
  // pixel size and the known render DPI.
  const [pageWidthPts, setPageWidthPts] = useState(0)
  const [pageHeightPts, setPageHeightPts] = useState(0)

  // Accumulated redactions across all pages, in PDF points (top-left origin).
  const [rects, setRects] = useState<RedactionRect[]>([])
  const [draft, setDraft] = useState<DraftRect | null>(null)

  const [applying, setApplying] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [confirming, setConfirming] = useState(false)
  const [notes, setNotes] = useState('')

  const overlayRef = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)

  // Render the current page at the fixed redaction DPI.
  useEffect(() => {
    let cancelled = false
    /* eslint-disable react-hooks/set-state-in-effect */
    setLoading(true)
    setRenderUrl(null)
    /* eslint-enable react-hooks/set-state-in-effect */
    ;(async () => {
      try {
        const url = await invoke<string>('render_page', { path: filePath, pageIndex, dpi: REDACT_DPI })
        if (!cancelled) setRenderUrl(url)
      } catch (e) {
        if (!cancelled) setError(`Failed to render page: ${String(e)}`)
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()
    return () => { cancelled = true }
  }, [filePath, pageIndex])

  const onImageLoad = useCallback((e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget
    // points = pixels * 72 / dpi
    setPageWidthPts((img.naturalWidth * 72) / REDACT_DPI)
    setPageHeightPts((img.naturalHeight * 72) / REDACT_DPI)
  }, [])

  const fractionFromEvent = useCallback((clientX: number, clientY: number): { x: number; y: number } | null => {
    const el = overlayRef.current
    if (!el) return null
    const rect = el.getBoundingClientRect()
    if (rect.width === 0 || rect.height === 0) return null
    const x = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width))
    const y = Math.min(1, Math.max(0, (clientY - rect.top) / rect.height))
    return { x, y }
  }, [])

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    const f = fractionFromEvent(e.clientX, e.clientY)
    if (!f) return
    dragging.current = true
    setDraft({ x0: f.x, y0: f.y, x1: f.x, y1: f.y })
  }, [fractionFromEvent])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragging.current) return
    const f = fractionFromEvent(e.clientX, e.clientY)
    if (!f) return
    setDraft((d) => (d ? { ...d, x1: f.x, y1: f.y } : null))
  }, [fractionFromEvent])

  const commitDraft = useCallback(() => {
    if (!dragging.current) return
    dragging.current = false
    setDraft((d) => {
      if (d && pageWidthPts > 0 && pageHeightPts > 0) {
        const fx0 = Math.min(d.x0, d.x1)
        const fy0 = Math.min(d.y0, d.y1)
        const fw = Math.abs(d.x1 - d.x0)
        const fh = Math.abs(d.y1 - d.y0)
        if (fw >= MIN_FRACTION && fh >= MIN_FRACTION) {
          const newRect: RedactionRect = {
            page: pageIndex,
            x: fx0 * pageWidthPts,
            y: fy0 * pageHeightPts,
            width: fw * pageWidthPts,
            height: fh * pageHeightPts,
          }
          setRects((prev) => [...prev, newRect])
        }
      }
      return null
    })
  }, [pageIndex, pageWidthPts, pageHeightPts])

  const removeRect = useCallback((index: number) => {
    setRects((prev) => prev.filter((_, i) => i !== index))
  }, [])

  const applyRedactions = useCallback(async () => {
    if (rects.length === 0) return
    setApplying(true)
    setError(null)
    try {
      const outputPath = deriveOutputPath(filePath)
      const result = await invoke<RedactionResult>('redact_pdf', {
        path: filePath,
        outputPath,
        redactions: rects,
        operatorName: '',
        notes,
      })
      onExported(result.output_path)
    } catch (e) {
      setError(`Redaction failed: ${String(e)}`)
    } finally {
      setApplying(false)
      setConfirming(false)
    }
  }, [filePath, rects, notes, onExported])

  // Keyboard: Escape closes (or cancels confirm), Delete removes the last rect.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (confirming) setConfirming(false)
        else onClose()
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        setRects((prev) => prev.slice(0, -1))
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [confirming, onClose])

  const pageRects = rects.filter((r) => r.page === pageIndex)
  const draftStyle = draft
    ? {
        left: `${Math.min(draft.x0, draft.x1) * 100}%`,
        top: `${Math.min(draft.y0, draft.y1) * 100}%`,
        width: `${Math.abs(draft.x1 - draft.x0) * 100}%`,
        height: `${Math.abs(draft.y1 - draft.y0) * 100}%`,
      }
    : null

  return (
    <div className="redact-layer" role="region" aria-label="Redaction">
      <div className="redact-toolbar" role="toolbar" aria-label="Redaction controls">
        <button className="btn btn-secondary" onClick={onClose} aria-label="Exit redaction">← Exit</button>
        <span className="redact-mode-badge" aria-live="polite">Redaction Mode</span>
        <nav className="redact-nav" aria-label="Page navigation">
          <button
            disabled={pageIndex <= 0}
            onClick={() => setPageIndex((p) => Math.max(0, p - 1))}
            aria-label="Previous page"
          >◀</button>
          <span aria-live="polite">Page {pageIndex + 1} / {pageCount}</span>
          <button
            disabled={pageIndex >= pageCount - 1}
            onClick={() => setPageIndex((p) => Math.min(pageCount - 1, p + 1))}
            aria-label="Next page"
          >▶</button>
        </nav>
        <span className="redact-count">{rects.length} redaction{rects.length === 1 ? '' : 's'}</span>
        <button
          className="btn btn-primary"
          disabled={rects.length === 0 || applying}
          onClick={() => setConfirming(true)}
        >
          {applying ? 'Applying…' : 'Apply & Export'}
        </button>
      </div>

      {error && (
        <div className="redact-error" role="alert">
          <span>{error}</span>
          <button onClick={() => setError(null)} aria-label="Dismiss error">✕</button>
        </div>
      )}

      <div className="redact-stage">
        {loading && <div className="redact-loading" role="status">Rendering…</div>}
        {renderUrl && (
          <div className="redact-image-wrap">
            <img
              src={convertFileSrc(renderUrl)}
              alt={`Page ${pageIndex + 1}`}
              onLoad={onImageLoad}
              draggable={false}
            />
            <div
              ref={overlayRef}
              className="redact-overlay"
              role="application"
              aria-label="Draw redaction rectangles"
              onMouseDown={handleMouseDown}
              onMouseMove={handleMouseMove}
              onMouseUp={commitDraft}
              onMouseLeave={commitDraft}
            >
              {pageRects.map((r, i) => {
                const globalIndex = rects.indexOf(r)
                if (pageWidthPts === 0 || pageHeightPts === 0) return null
                const style = {
                  left: `${(r.x / pageWidthPts) * 100}%`,
                  top: `${(r.y / pageHeightPts) * 100}%`,
                  width: `${(r.width / pageWidthPts) * 100}%`,
                  height: `${(r.height / pageHeightPts) * 100}%`,
                }
                return (
                  <div key={i} className="redact-box" style={style}>
                    <button
                      className="redact-box-remove"
                      onClick={(e) => { e.stopPropagation(); removeRect(globalIndex) }}
                      aria-label={`Remove redaction ${i + 1}`}
                    >✕</button>
                  </div>
                )
              })}
              {draftStyle && <div className="redact-box redact-box--draft" style={draftStyle} />}
            </div>
          </div>
        )}
      </div>

      {confirming && (
        <div className="redact-confirm-backdrop" role="dialog" aria-modal="true" aria-label="Confirm redaction">
          <div className="redact-confirm">
            <h3>Apply {rects.length} redaction{rects.length === 1 ? '' : 's'}?</h3>
            <p className="redact-warning">
              This action is <strong>irreversible</strong>. The redacted content is permanently
              painted over and a tamper-evident audit record is written. The original file is
              not modified — a new <code>.redacted.pdf</code> is created.
            </p>
            <label className="redact-notes-label">
              Notes (optional)
              <input
                type="text"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="e.g. redacted client PII"
              />
            </label>
            <div className="redact-confirm-actions">
              <button className="btn btn-secondary" onClick={() => setConfirming(false)} disabled={applying}>Cancel</button>
              <button className="btn btn-primary" onClick={applyRedactions} disabled={applying}>
                {applying ? 'Applying…' : 'Confirm & Export'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
