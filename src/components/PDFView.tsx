import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { PdfSummary, CombinedPreflightResult, TextMatch } from '../types'
import PreflightReport from './preflight/PreflightReport'
import PdfInspector from './preflight/PdfInspector'
import ColorConversionPanel from './preflight/ColorConversionPanel'
import MakePdfXWizard from './preflight/MakePdfXWizard'
import CertifiedVersionPanel from './preflight/CertifiedVersionPanel'
import OCRPanel from './preflight/OCRPanel'
import RedactionLayer from './RedactionLayer'
import { useAnnotations } from './useAnnotations'
import { AnnotationToolbar, AnnotationOverlay } from './AnnotationLayer'
import { makeKeyDownHandler, buildShortcuts, formatShortcut, type ShortcutHandlers } from './preflight/keyboardShortcuts'
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
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setThumbnails({})
    const max = Math.min(pageCount, 20)
    let cancelled = false

    async function loadThumbs() {
      const results: Record<number, string> = {}
      const MAX_CONCURRENT = 3
      let nextIdx = 0

      const loadOne = async () => {
        if (nextIdx >= max || cancelled) return
        const idx = nextIdx++

        try {
          const url = await invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: idx, widthPx: 120 })
          if (!cancelled) results[idx] = url
        } catch {
          // ignore per-thumbnail errors
        } finally {
          if (nextIdx < max && !cancelled) await loadOne()
        }
      }

      const promises = Array.from({ length: Math.min(MAX_CONCURRENT, max) }, () => loadOne())
      await Promise.all(promises)

      if (!cancelled) setThumbnails(results)
    }

    loadThumbs()
    return () => { cancelled = true }
  }, [filePath, pageCount])

  return (
    <div className="thumb-strip" ref={containerRef} role="tablist" aria-label={t('pdf.recent')}>
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
            if (e.key === 'ArrowRight' && i < Math.min(pageCount, 20) - 1) {
              e.preventDefault()
              const next = containerRef.current?.querySelector<HTMLButtonElement>(`.thumb-item:nth-child(${i + 2})`)
              next?.focus()
            } else if (e.key === 'ArrowLeft' && i > 0) {
              e.preventDefault()
              const prev = containerRef.current?.querySelector<HTMLButtonElement>(`.thumb-item:nth-child(${i})`)
              prev?.focus()
            }
          }}
        >
          {thumbnails[i] ? (
            <img src={convertFileSrc(thumbnails[i])} alt={`Page ${i + 1}`} />
          ) : (
            <div className="thumb-placeholder" aria-hidden="true">{i + 1}</div>
          )}
        </button>
      ))}
    </div>
  )
}

function rgbToHex(r: number, g: number, b: number): string {
  return (
    '#' +
    r.toString(16).padStart(2, '0') +
    g.toString(16).padStart(2, '0') +
    b.toString(16).padStart(2, '0')
  );
}

function PageViewer({ filePath, pageIndex }: { filePath: string; pageIndex: number }) {
  const [renderUrl, setRenderUrl] = useState<string | null>(null)
  const [zoom, setZoom] = useState(100)
  const [loading, setLoading] = useState(false)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [eyedropperActive, setEyedropperActive] = useState(false)
  const [sampledColor, setSampledColor] = useState<{ r: number; g: number; b: number; hex: string } | null>(null)
  const [showPlateView, setShowPlateView] = useState(false)
  const [showAnnotations, setShowAnnotations] = useState(false)
  const [pageWidthPts, setPageWidthPts] = useState(0)
  const [pageHeightPts, setPageHeightPts] = useState(0)
  const annotState = useAnnotations(filePath, pageIndex, pageWidthPts, pageHeightPts)
  const imgRef = useRef<HTMLImageElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

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

  // Fullscreen: enter / leave on the page-viewer container so the
  // browser's ESC handler works for free.
  const toggleFullscreen = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    if (!document.fullscreenElement) {
      el.requestFullscreen?.().then(() => setIsFullscreen(true)).catch(() => {});
    } else {
      document.exitFullscreen?.().then(() => setIsFullscreen(false)).catch(() => {});
    }
  }, []);

  useEffect(() => {
    const handler = () => setIsFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener('fullscreenchange', handler);
    return () => document.removeEventListener('fullscreenchange', handler);
  }, []);

  // Track page dimensions in PDF points so the annotation overlay can
  // convert between fraction-of-image and PDF user-space coordinates.
  const handleAnnotationImageLoad = useCallback((e: React.SyntheticEvent<HTMLImageElement>) => {
    if (!showAnnotations) return
    const img = e.currentTarget
    const dpi = Math.round(72 * zoom / 100)
    setPageWidthPts((img.naturalWidth * 72) / dpi)
    setPageHeightPts((img.naturalHeight * 72) / dpi)
  }, [showAnnotations, zoom])

  // Re-compute dimensions when zoom or annotation mode changes.
  useEffect(() => {
    if (!showAnnotations || !imgRef.current || imgRef.current.naturalWidth === 0) return
    const dpi = Math.round(72 * zoom / 100)
    setPageWidthPts((imgRef.current.naturalWidth * 72) / dpi)
    setPageHeightPts((imgRef.current.naturalHeight * 72) / dpi)
  }, [showAnnotations, zoom])

  // Eyedropper: when active, the next click on the page reads
  // the pixel at the click coordinates from a 1×1 canvas.
  const handleImageClick = useCallback(
    (e: React.MouseEvent<HTMLImageElement>) => {
      if (!eyedropperActive || !imgRef.current) return;
      const img = imgRef.current;
      const rect = img.getBoundingClientRect();
      const xPx = Math.floor(((e.clientX - rect.left) / rect.width) * img.naturalWidth);
      const yPx = Math.floor(((e.clientY - rect.top) / rect.height) * img.naturalHeight);
      const canvas = document.createElement('canvas');
      canvas.width = 1;
      canvas.height = 1;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      try {
        ctx.drawImage(img, xPx, yPx, 1, 1, 0, 0, 1, 1);
        const data = ctx.getImageData(0, 0, 1, 1).data;
        const r = data[0];
        const g = data[1];
        const b = data[2];
        setSampledColor({ r, g, b, hex: rgbToHex(r, g, b) });
      } catch {
        // Cross-origin images throw on getImageData; we silently fail
        // and leave sampledColor unchanged.
      }
      setEyedropperActive(false);
    },
    [eyedropperActive]
  );

  return (
    <div
      ref={containerRef}
      className={`page-viewer${isFullscreen ? ' page-viewer--fullscreen' : ''}`}
      role="region"
      aria-label={`Page ${pageIndex + 1}`}
    >
      <div className="page-toolbar" role="toolbar" aria-label={t('pdf.tools')}>
        <button aria-label={t('pdf.zoom_out')} onClick={() => setZoom((z) => Math.max(25, z - 25))}>−</button>
        <span className="zoom-label" aria-live="polite">{zoom}%</span>
        <button aria-label={t('pdf.zoom_in')} onClick={() => setZoom((z) => Math.min(400, z + 25))}>+</button>
        <button aria-label={t('pdf.fit_width')} onClick={() => setZoom(100)}>{t('pdf.fit_width')}</button>
        <button
          aria-label={isFullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}
          className={isFullscreen ? 'btn-active' : ''}
          onClick={toggleFullscreen}
          title="Fullscreen (F)"
        >
          ⛶
        </button>
        <button
          aria-label="Eyedropper"
          className={eyedropperActive ? 'btn-active' : ''}
          onClick={() => {
            setEyedropperActive((v) => !v);
            setSampledColor(null);
          }}
          title="Eyedropper — click the page to sample a color"
        >
          💧
        </button>
        <button
          aria-label="Plate view"
          className={showPlateView ? 'btn-active' : ''}
          onClick={() => setShowPlateView((v) => !v)}
          title="Plate view — separations preview"
        >
          ⬚
        </button>
        <button
          aria-label={showAnnotations ? 'Hide annotations' : 'Show annotations'}
          className={showAnnotations ? 'btn-active' : ''}
          onClick={() => setShowAnnotations((v) => !v)}
          title="Annotations"
        >
          🖊
        </button>
        {eyedropperActive && (
          <span className="eyedropper-hint" role="status">
            Click the page to sample…
          </span>
        )}
        {sampledColor && (
          <span
            className="eyedropper-swatch"
            role="status"
            style={{ backgroundColor: sampledColor.hex }}
            title={`rgb(${sampledColor.r}, ${sampledColor.g}, ${sampledColor.b})`}
          >
            {sampledColor.hex}
          </span>
        )}
      </div>
      {showAnnotations && <AnnotationToolbar state={annotState} />}
      <div className="page-canvas" role="img" aria-label={`Page ${pageIndex + 1}`}>
        {loading && <div className="page-loading" role="status">{t('pdf.rendering')}</div>}
        {renderUrl && (
          <div style={{ position: 'relative', display: 'inline-block', maxWidth: `${zoom}%` }}>
            <img
              ref={imgRef}
              src={convertFileSrc(renderUrl)}
              alt={`Page ${pageIndex + 1}`}
              style={{
                display: 'block',
                width: '100%',
                cursor: eyedropperActive ? 'crosshair' : 'default',
              }}
              onClick={handleImageClick}
              onLoad={handleAnnotationImageLoad}
              crossOrigin="anonymous"
            />
            {showAnnotations && pageWidthPts > 0 && <AnnotationOverlay state={annotState} />}
          </div>
        )}
        {showPlateView && renderUrl && (
          <div className="plate-view-overlay" aria-label="Plate view (separations preview)">
            <div className="plate-view-label">Plate View</div>
            <div className="plate-view-channels">
              {(['C', 'M', 'Y', 'K'] as const).map((c) => (
                <div key={c} className={`plate-view-channel plate-view-channel--${c.toLowerCase()}`}>
                  <span className="plate-view-channel-letter">{c}</span>
                </div>
              ))}
            </div>
            <p className="plate-view-help">
              Conceptual separations preview. Real CMYK plate generation
              requires a per-channel render through pdfium; this overlay
              indicates which separations would be produced.
            </p>
          </div>
        )}
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
  const [showOCR, setShowOCR] = useState(false)
  const [showRedact, setShowRedact] = useState(false)
  const [redactNotice, setRedactNotice] = useState<string | null>(null)
  const [showFind, setShowFind] = useState(false)
  const [findQuery, setFindQuery] = useState('')
  const [findResult, setFindResult] = useState<string | null>(null)
  const [showHelp, setShowHelp] = useState(false)
  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { setCurrentPage(0); setShowViewer(false); setPreflightResult(null); setShowReport(false); setShowRedact(false); setRedactNotice(null) }, [summary?.file_path])

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

  const handleFind = useCallback(async () => {
    if (!summary) return
    setShowFind(true)
    setFindResult(null)
  }, [summary])

  const submitFind = useCallback(async () => {
    if (!summary || !findQuery.trim()) return
    try {
      const matches = await invoke<TextMatch[]>('search_text', { path: summary.file_path, query: findQuery, caseSensitive: false })
      setFindResult(matches.length === 0
        ? 'No matches found.'
        : `${matches.length} match${matches.length === 1 ? '' : 'es'} on page ${matches[0].page_index + 1}.`)
      if (matches.length > 0 && showViewer) {
        setCurrentPage(matches[0].page_index)
      }
    } catch (e) {
      setFindResult(`Search failed: ${e}`)
    }
  }, [summary, findQuery, showViewer])

  const handlers = useMemo(() => ({
    onFind: handleFind,
    onSaveProfile: () => onSaveJob(),
    onRunProfile: runFullPreflight,
    onOpen: onOpenFile,
    onRunPreflight: runFullPreflight,
    onNextPage: () => showViewer && setCurrentPage(p => Math.min((summary?.page_count ?? 1) - 1, p + 1)),
    onPrevPage: () => showViewer && setCurrentPage(p => Math.max(0, p - 1)),
    onFirstPage: () => showViewer && setCurrentPage(0),
    onLastPage: () => showViewer && setCurrentPage((summary?.page_count ?? 1) - 1),
    onFullscreen: () => {
      if (document.fullscreenElement) {
        document.exitFullscreen().catch(() => undefined)
      } else {
        document.documentElement.requestFullscreen().catch(() => undefined)
      }
    },
    onHelp: () => setShowHelp(true),
  } satisfies ShortcutHandlers), [handleFind, onSaveJob, runFullPreflight, onOpenFile, showViewer, summary?.page_count])

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    makeKeyDownHandler(handlers)(e)
  }, [handlers])

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleKeyDown])

  useEffect(() => {
    if (!showFind) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setShowFind(false)
        setFindResult(null)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [showFind])

  useEffect(() => {
    if (!showHelp) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' || e.key === '?') setShowHelp(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [showHelp])

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
              <button className="btn btn-secondary" onClick={() => setShowOCR(!showOCR)}>
                {showOCR ? 'Hide OCR' : 'Optical Character Recognition'}
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
            {showOCR && (
              <OCRPanel filePath={summary.file_path} pageCount={summary.page_count} />
            )}
          </div>
        ) : showRedact ? (
          <RedactionLayer
            filePath={summary.file_path}
            pageCount={summary.page_count}
            initialPage={currentPage}
            onClose={() => setShowRedact(false)}
            onExported={(outputPath) => {
              setShowRedact(false)
              setRedactNotice(`Redacted PDF saved to ${outputPath}`)
            }}
          />
        ) : (
          <div className="pdf-viewer-section">
            <div className="pdf-viewer-header">
              <button className="btn btn-secondary" onClick={() => setShowViewer(false)} aria-label={t('pdf.back')}>← {t('pdf.back')}</button>
              <span className="pdf-viewer-title">{summary.file_name}</span>
              <button className="btn btn-secondary" onClick={runFullPreflight} disabled={runningPreflight} style={{ marginRight: 8 }}>
                {runningPreflight ? '...' : t('pdf.preflight')}
              </button>
              <button className="btn btn-secondary" onClick={() => setShowRedact(true)} style={{ marginRight: 8 }} aria-label="Redact">
                Redact
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
            {redactNotice && (
              <div className="pdf-error-banner" style={{ background: '#e7f6ec', color: '#1e7e34' }} role="status">
                <span>{redactNotice}</span>
                <button onClick={() => setRedactNotice(null)} aria-label={t('common.remove')}>✕</button>
              </div>
            )}
            <PageViewer filePath={summary.file_path} pageIndex={currentPage} />
          </div>
        )}
        {showFind && (
          <div className="pdf-find-overlay" role="dialog" aria-label="Find text">
            <div className="pdf-find-panel">
              <label htmlFor="pdf-find-input" className="pdf-label">Find in document</label>
              <input
                id="pdf-find-input"
                className="form-input"
                autoFocus
                value={findQuery}
                onChange={(e) => setFindQuery(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') submitFind() }}
                placeholder="Search text..."
              />
              {findResult && <p className="pdf-find-result">{findResult}</p>}
              <div className="pdf-find-actions">
                <button className="btn btn-secondary" onClick={() => { setShowFind(false); setFindResult(null) }}>Close</button>
                <button className="btn btn-primary" onClick={submitFind}>Find</button>
              </div>
            </div>
          </div>
        )}
        {showHelp && (
          <div className="pdf-find-overlay" role="dialog" aria-label="Keyboard shortcuts">
            <div className="pdf-find-panel pdf-help-panel">
              <h4>Keyboard shortcuts</h4>
              <ul>
                {buildShortcuts().map((s, i) => (
                  <li key={i}>
                    <kbd>{formatShortcut(s)}</kbd>
                    <span>{s.description}</span>
                  </li>
                ))}
              </ul>
              <div className="pdf-find-actions">
                <button className="btn btn-primary" onClick={() => setShowHelp(false)}>Close</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
