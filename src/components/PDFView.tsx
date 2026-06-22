import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PdfSummary, CombinedPreflightResult } from '../types'
import PreflightReport from './preflight/PreflightReport'
import PdfInspector from './preflight/PdfInspector'
import ColorConversionPanel from './preflight/ColorConversionPanel'
import MakePdfXWizard from './preflight/MakePdfXWizard'
import CertifiedVersionPanel from './preflight/CertifiedVersionPanel'
import { t } from '../i18n'
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
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setThumbnails({})
    const max = Math.min(pageCount, 20)
    let cancelled = false

    async function loadThumbs() {
      const results: Record<number, string> = {}
      for (let i = 0; i < max; i++) {
        try {
          const url = await invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: i, widthPx: 120 })
          if (cancelled) break
          results[i] = url
        } catch {
          // ignore per-thumbnail errors
        }
      }
      if (!cancelled) setThumbnails(results)
    }

    loadThumbs()
    return () => { cancelled = true }
  }, [filePath, pageCount])

  return (
    <div className="thumb-strip" role="tablist" aria-label={t('pdf.recent')}>
      {Array.from({ length: Math.min(pageCount, 20) }, (_, i) => (
        <button
          key={i}
          role="tab"
          aria-selected={i === currentPage}
          aria-label={`Page ${i + 1}`}
          tabIndex={i === currentPage ? 0 : -1}
          className={`thumb-item ${i === currentPage ? 'thumb-item--active' : ''}`}
          onClick={() => onSelectPage(i)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowRight') {
              e.preventDefault()
              const next = document.querySelector<HTMLButtonElement>(`.thumb-item:nth-child(${i + 2})`)
              next?.focus()
            } else if (e.key === 'ArrowLeft') {
              e.preventDefault()
              const prev = document.querySelector<HTMLButtonElement>(`.thumb-item:nth-child(${i})`)
              prev?.focus()
            }
          }}
        >
          {thumbnails[i] ? (
            <img src={`file://${thumbnails[i]}`} alt={`Page ${i + 1}`} />
          ) : (
            <div className="thumb-placeholder" aria-hidden="true">{i + 1}</div>
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
    let cancelled = false
    /* eslint-disable react-hooks/set-state-in-effect */
    setLoading(true)
    setRenderUrl(null)
    /* eslint-enable react-hooks/set-state-in-effect */
    const dpi = Math.round(72 * zoom / 100)

    ;(async () => {
      try {
        const url = await invoke<string>('render_page', { path: filePath, pageIndex, dpi })
        if (!cancelled) setRenderUrl(url)
      } catch {
        // ignore render errors
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()

    return () => { cancelled = true }
  }, [filePath, pageIndex, zoom])

  return (
    <div className="page-viewer" role="region" aria-label={`Page ${pageIndex + 1}`}>
      <div className="page-toolbar" role="toolbar" aria-label={t('pdf.tools')}>
        <button aria-label={t('pdf.zoom_out')} onClick={() => setZoom((z) => Math.max(25, z - 25))}>−</button>
        <span className="zoom-label" aria-live="polite">{zoom}%</span>
        <button aria-label={t('pdf.zoom_in')} onClick={() => setZoom((z) => Math.min(400, z + 25))}>+</button>
        <button aria-label={t('pdf.fit_width')} onClick={() => setZoom(100)}>{t('pdf.fit_width')}</button>
      </div>
      <div className="page-canvas" role="img" aria-label={`Page ${pageIndex + 1}`}>
        {loading && <div className="page-loading" role="status">{t('pdf.rendering')}</div>}
        {renderUrl && <img src={`file://${renderUrl}`} alt={`Page ${pageIndex + 1}`} style={{ maxWidth: `${zoom}%` }} />}
      </div>
    </div>
  )
}

export default function PDFView({ summary, jobs, onOpenFile, onSaveJob, onDeleteJob, onLoadJob, error, onClearError }: PDFViewProps) {
  const [currentPage, setCurrentPage] = useState(0)
  const [showViewer, setShowViewer] = useState(false)
  const [preflightResult, setPreflightResult] = useState<CombinedPreflightResult | null>(null)
  const [runningPreflight, setRunningPreflight] = useState(false)
  const [showReport, setShowReport] = useState(false)
  const [showInspector, setShowInspector] = useState(false)
  const [showConversion, setShowConversion] = useState(false)
  const [showWizard, setShowWizard] = useState(false)
  const [showCertified, setShowCertified] = useState(false)
  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { setCurrentPage(0); setShowViewer(false); setPreflightResult(null); setShowReport(false) }, [summary?.file_path])

  const runFullPreflight = useCallback(async () => {
    if (!summary) return
    setRunningPreflight(true)
    try {
      const result = await invoke<CombinedPreflightResult>('check_full_preflight', { path: summary.file_path })
      setPreflightResult(result)
      setShowReport(true)
    } catch (e) {
      console.error('Preflight failed:', e)
    } finally {
      setRunningPreflight(false)
    }
  }, [summary])

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'ArrowLeft' && showViewer) {
      setCurrentPage(p => Math.max(0, p - 1))
    } else if (e.key === 'ArrowRight' && showViewer) {
      setCurrentPage(p => Math.min((summary?.page_count ?? 1) - 1, p + 1))
    } else if (e.key === '+' || e.key === '=') {
      // Zoom in handled by PageViewer's internal state, but we can trigger a re-render
    } else if (e.key === '-') {
      // Zoom out
    } else if (e.key === 'o' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      onOpenFile()
    } else if (e.key === 'r' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      runFullPreflight()
    } else if (e.key === 'Escape') {
      setShowReport(false)
    }
  }, [showViewer, summary?.page_count, onOpenFile, runFullPreflight])

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleKeyDown])

  return (
    <div className="pdf-view" role="main" aria-label={t('pdf.tools')}>
      <div className="pdf-sidebar" role="complementary" aria-label={t('pdf.recent')}>
        <h3>{t('pdf.recent')}</h3>
        <button className="btn btn-primary pdf-open-btn" onClick={onOpenFile} title={t('pdf.open.shortcut')} aria-label={t('pdf.open')}>
          {t('pdf.open')}
        </button>
        <div className="pdf-job-list" role="list" aria-label={t('pdf.recent')}>
          {jobs.length === 0 && <p className="pdf-empty">{t('pdf.no_recent')}</p>}
          {jobs.map((job) => (
            <div key={job.id} className="pdf-job-item" role="listitem">
              <button className="pdf-job-name" onClick={() => onLoadJob(job.id)} aria-label={`${job.file_name}`}>
                {job.file_name}
              </button>
              <span className="pdf-job-meta">{job.page_count}p</span>
              <button className="pdf-job-delete" onClick={() => onDeleteJob(job.id)} aria-label={t('common.remove')}>
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
          <div className="pdf-empty-state" role="region" aria-label={t('pdf.tools')}>
            <h2>{t('pdf.tools')}</h2>
            <p>{t('pdf.tools_desc')}</p>
            <button className="btn btn-primary" onClick={onOpenFile}>{t('pdf.open')}</button>
          </div>
        ) : !showViewer ? (
          <div className="pdf-summary-card" role="region" aria-label={summary.file_name}>
            <div className="pdf-summary-header">
              <h2>{summary.file_name}</h2>
              {summary.is_encrypted && <span className="pdf-badge pdf-badge-error" aria-label="Encrypted">Encrypted</span>}
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
              <button className="btn btn-primary" onClick={() => setShowViewer(true)}>{t('pdf.view_pages')}</button>
              <button className="btn btn-secondary pdf-save-btn" onClick={onSaveJob}>{t('pdf.save_history')}</button>
              <button className="btn btn-secondary" onClick={runFullPreflight} disabled={runningPreflight}>
                {runningPreflight ? t('pdf.running_preflight') : t('pdf.run_preflight')}
              </button>
            </div>

            {showReport && preflightResult && (
              <PreflightReport
                filePath={summary.file_path}
                result={preflightResult}
                jobId={summary.id ?? null}
                onSaved={() => { }}
              />
            )}
            <div className="pdf-summary-actions" style={{ marginTop: 12 }}>
              <button className="btn btn-secondary" onClick={() => setShowInspector(!showInspector)}>
                {showInspector ? 'Hide Inspector' : 'Show Inspector'}
              </button>
              <button className="btn btn-secondary" onClick={() => setShowConversion(!showConversion)}>
                {showConversion ? 'Hide Conversion' : 'RGB→CMYK Conversion'}
              </button>
              <button className="btn btn-secondary" onClick={() => setShowWizard(!showWizard)}>
                {showWizard ? 'Hide Wizard' : 'Make PDF/X Wizard'}
              </button>
              <button className="btn btn-secondary" onClick={() => setShowCertified(!showCertified)}>
                {showCertified ? 'Hide Versions' : 'Certified PDF'}
              </button>
            </div>
            {showInspector && (
              <PdfInspector filePath={summary.file_path} />
            )}
            {showConversion && (
              <ColorConversionPanel filePath={summary.file_path} preflightResult={preflightResult} />
            )}
            {showWizard && (
              <MakePdfXWizard
                filePath={summary.file_path}
                preflightResult={preflightResult}
                onRerunPreflight={runFullPreflight}
              />
            )}
            {showCertified && (
              <CertifiedVersionPanel jobId={summary.id ?? null} filePath={summary.file_path} />
            )}
          </div>
        ) : (
          <div className="pdf-viewer-section">
            <div className="pdf-viewer-header">
              <button className="btn btn-secondary" onClick={() => setShowViewer(false)} aria-label={t('pdf.back')}>← {t('pdf.back')}</button>
              <span className="pdf-viewer-title">{summary.file_name}</span>
              <button className="btn btn-secondary" onClick={runFullPreflight} disabled={runningPreflight} style={{ marginRight: 8 }}>
                {runningPreflight ? '...' : t('pdf.preflight')}
              </button>
              <nav className="pdf-nav" aria-label={t('pdf.recent')}>
                <button
                  disabled={currentPage <= 0}
                  onClick={() => setCurrentPage((p) => Math.max(0, p - 1))}
                  aria-label={t('pdf.prev_page')}
                  tabIndex={0}
                >◀</button>
                <span aria-live="polite">{t('pdf.page_of', { current: currentPage + 1, total: summary.page_count })}</span>
                <button
                  disabled={currentPage >= summary.page_count - 1}
                  onClick={() => setCurrentPage((p) => Math.min(summary.page_count - 1, p + 1))}
                  aria-label={t('pdf.next_page')}
                  tabIndex={0}
                >▶</button>
              </nav>
            </div>
            <PageViewer filePath={summary.file_path} pageIndex={currentPage} />
          </div>
        )}
      </div>
    </div>
  )
}
