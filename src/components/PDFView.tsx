import { useState, useEffect, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PdfSummary, FontFinding, PageBoxFinding, ImageResolutionFinding } from '../types'
import './PDFView.css'

interface PDFViewProps {
  summary: PdfSummary | null
  jobs: PdfSummary[]
  onOpenFile: () => Promise<void>
  onSaveJob: () => Promise<void>
  onDeleteJob: (id: number) => Promise<void>
  onLoadJob: (id: number) => Promise<void>
  error: string | null
  onClearError: () => void
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function ThumbnailStrip({ filePath, pageCount, currentPage, onSelectPage }: {
  filePath: string
  pageCount: number
  currentPage: number
  onSelectPage: (n: number) => void
}) {
  const [thumbnails, setThumbnails] = useState<Record<number, string>>({})

  useEffect(() => {
    setThumbnails({})
    const max = Math.min(pageCount, 20)
    let cancelled = false
    for (let i = 0; i < max; i++) {
      invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: i, widthPx: 120 })
        .then((url) => { if (!cancelled) setThumbnails((prev) => ({ ...prev, [i]: url })) })
        .catch(() => {})
    }
    return () => { cancelled = true }
  }, [filePath, pageCount])

  return (
    <div className="thumb-strip">
      {Array.from({ length: Math.min(pageCount, 20) }, (_, i) => (
        <button
          key={i}
          className={`thumb-item ${i === currentPage ? 'thumb-item--active' : ''}`}
          onClick={() => onSelectPage(i)}
        >
          {thumbnails[i] ? (
            <img src={`file://${thumbnails[i]}`} alt={`Page ${i + 1}`} />
          ) : (
            <div className="thumb-placeholder">{i + 1}</div>
          )}
        </button>
      ))}
    </div>
  )
}

function PageViewer({ filePath, pageIndex }: { filePath: string; pageIndex: number }) {
  const [renderUrl, setRenderUrl] = useState<string | null>(null)
  const [zoom, setZoom] = useState(100)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    setLoading(true)
    setRenderUrl(null)
    const dpi = Math.round(72 * zoom / 100)
    invoke<string>('render_page', { path: filePath, pageIndex, dpi })
      .then((url) => setRenderUrl(url))
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [filePath, pageIndex, zoom])

  return (
    <div className="page-viewer">
      <div className="page-toolbar">
        <button onClick={() => setZoom((z) => Math.max(25, z - 25))}>−</button>
        <span className="zoom-label">{zoom}%</span>
        <button onClick={() => setZoom((z) => Math.min(400, z + 25))}>+</button>
        <button onClick={() => setZoom(100)}>Fit</button>
      </div>
      <div className="page-canvas">
        {loading && <div className="page-loading">Rendering...</div>}
        {renderUrl && <img src={`file://${renderUrl}`} alt={`Page ${pageIndex + 1}`} style={{ maxWidth: `${zoom}%` }} />}
      </div>
    </div>
  )
}

export default function PDFView({ summary, jobs, onOpenFile, onSaveJob, onDeleteJob, onLoadJob, error, onClearError }: PDFViewProps) {
  const [currentPage, setCurrentPage] = useState(0)
  const [showViewer, setShowViewer] = useState(false)
  const [fontFindings, setFontFindings] = useState<FontFinding[]>([])
  const [boxFindings, setBoxFindings] = useState<PageBoxFinding[]>([])
  const [imageFindings, setImageFindings] = useState<ImageResolutionFinding[]>([])
  const [minDpi, setMinDpi] = useState(150)
  const [sortByDpi, setSortByDpi] = useState(true)

  useEffect(() => { setCurrentPage(0); setShowViewer(false) }, [summary?.file_path])

  useEffect(() => {
    if (!summary) { setFontFindings([]); setBoxFindings([]); setImageFindings([]); return }
    invoke<FontFinding[]>('check_fonts', { path: summary.file_path })
      .then(setFontFindings)
      .catch(() => setFontFindings([]))
    invoke<PageBoxFinding[]>('check_page_boxes', { path: summary.file_path })
      .then(setBoxFindings)
      .catch(() => setBoxFindings([]))
    invoke<ImageResolutionFinding[]>('check_image_resolution', { path: summary.file_path })
      .then(r => setImageFindings(r ?? []))
      .catch(() => setImageFindings([]))
  }, [summary])

  return (
    <div className="pdf-view">
      <div className="pdf-sidebar">
        <h3>Recent PDFs</h3>
        <button className="btn btn-primary pdf-open-btn" onClick={onOpenFile}>
          Open PDF
        </button>
        <div className="pdf-job-list">
          {jobs.length === 0 && <p className="pdf-empty">No recent files</p>}
          {jobs.map((job) => (
            <div key={job.id} className="pdf-job-item">
              <button className="pdf-job-name" onClick={() => onLoadJob(job.id)}>
                {job.file_name}
              </button>
              <span className="pdf-job-meta">{job.page_count}p</span>
              <button className="pdf-job-delete" onClick={() => onDeleteJob(job.id)} title="Remove">
                ✕
              </button>
            </div>
          ))}
        </div>
        {summary && (
          <ThumbnailStrip
            filePath={summary.file_path}
            pageCount={summary.page_count}
            currentPage={currentPage}
            onSelectPage={(n) => { setCurrentPage(n); setShowViewer(true) }}
          />
        )}
      </div>

      <div className="pdf-main">
        {error && (
          <div className="pdf-error-banner">
            <span>{error}</span>
            <button onClick={onClearError}>✕</button>
          </div>
        )}

        {!summary ? (
          <div className="pdf-empty-state">
            <h2>PDF Tools</h2>
            <p>Open a PDF to inspect and preflight it.</p>
            <button className="btn btn-primary" onClick={onOpenFile}>Open PDF</button>
          </div>
        ) : !showViewer ? (
          <div className="pdf-summary-card">
            <div className="pdf-summary-header">
              <h2>{summary.file_name}</h2>
              {summary.is_encrypted && <span className="pdf-badge pdf-badge-error">Encrypted</span>}
            </div>
            <div className="pdf-summary-grid">
              <div className="pdf-summary-item">
                <span className="pdf-label">Pages</span>
                <span className="pdf-value">{summary.page_count}</span>
              </div>
              <div className="pdf-summary-item">
                <span className="pdf-label">PDF Version</span>
                <span className="pdf-value">{summary.pdf_version}</span>
              </div>
              <div className="pdf-summary-item">
                <span className="pdf-label">File Size</span>
                <span className="pdf-value">{formatBytes(summary.file_size_bytes)}</span>
              </div>
              <div className="pdf-summary-item">
                <span className="pdf-label">Title</span>
                <span className="pdf-value">{summary.title || '—'}</span>
              </div>
              <div className="pdf-summary-item">
                <span className="pdf-label">Creator</span>
                <span className="pdf-value">{summary.creator || '—'}</span>
              </div>
              <div className="pdf-summary-item">
                <span className="pdf-label">Producer</span>
                <span className="pdf-value">{summary.producer || '—'}</span>
              </div>
            </div>
            <div className="pdf-summary-actions">
              <button className="btn btn-primary" onClick={() => setShowViewer(true)}>View Pages</button>
              <button className="btn btn-secondary pdf-save-btn" onClick={onSaveJob}>Save to History</button>
            </div>

            {(fontFindings.length > 0 || boxFindings.length > 0 || imageFindings.length > 0) && (
              <div className="pdf-preflight">
                <h3>Preflight Results</h3>
                {fontFindings.length > 0 && (
                  <div className="pdf-preflight-section">
                    <h4>Font Checks</h4>
                    {fontFindings.map((f, i) => (
                      <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
                        <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
                        <span className="pdf-finding-name">{f.font_name}</span>
                        <span className="pdf-finding-type">({f.font_type})</span>
                        <span className="pdf-finding-pages">p. {f.pages.join(', ')}</span>
                      </div>
                    ))}
                  </div>
                )}
                {boxFindings.length > 0 && (
                  <div className="pdf-preflight-section">
                    <h4>Page Box Checks</h4>
                    {boxFindings.map((f, i) => (
                      <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
                        <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
                        <span className="pdf-finding-name">P.{f.page} {f.box_type}</span>
                        <span className="pdf-finding-message">{f.message}</span>
                      </div>
                    ))}
                  </div>
                )}
                {imageFindings.length > 0 && (
                  <div className="pdf-preflight-section">
                    <div className="pdf-preflight-controls">
                      <h4>Image Resolution</h4>
                      <label className="dpi-slider">
                        Min DPI: {minDpi}
                        <input type="range" min="72" max="600" value={minDpi}
                          onChange={e => setMinDpi(Number(e.target.value))} />
                      </label>
                      <label className="dpi-sort">
                        <input type="checkbox" checked={sortByDpi}
                          onChange={e => setSortByDpi(e.target.checked)} />
                        Sort by DPI
                      </label>
                    </div>
                    {(sortByDpi ? [...imageFindings].sort((a, b) => a.effective_dpi - b.effective_dpi) : imageFindings).map((f, i) => {
                      const sev = f.effective_dpi < minDpi ? 'error' : f.effective_dpi < minDpi * 1.5 ? 'warning' : 'ok'
                      return (
                        <div key={i} className={`pdf-finding pdf-finding--${sev}`}>
                          <span className="pdf-finding-sev">{sev.toUpperCase()}</span>
                          <span className="pdf-finding-name">P.{f.page} {f.image_name}</span>
                          <span className="pdf-finding-type">{f.pixel_width}×{f.pixel_height}px / {f.color_space}</span>
                          <span className="pdf-finding-message">{f.effective_dpi.toFixed(0)} DPI — {f.message}</span>
                        </div>
                      )
                    })}
                  </div>
                )}
              </div>
            )}
          </div>
        ) : (
          <div className="pdf-viewer-section">
            <div className="pdf-viewer-header">
              <button className="btn btn-secondary" onClick={() => setShowViewer(false)}>← Back</button>
              <span className="pdf-viewer-title">{summary.file_name}</span>
              <div className="pdf-nav">
                <button
                  disabled={currentPage <= 0}
                  onClick={() => setCurrentPage((p) => Math.max(0, p - 1))}
                >◀</button>
                <span>Page {currentPage + 1} of {summary.page_count}</span>
                <button
                  disabled={currentPage >= summary.page_count - 1}
                  onClick={() => setCurrentPage((p) => Math.min(summary.page_count - 1, p + 1))}
                >▶</button>
              </div>
            </div>
            <PageViewer filePath={summary.file_path} pageIndex={currentPage} />
          </div>
        )}
      </div>
    </div>
  )
}
