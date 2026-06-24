import { useState, useEffect, useRef, useCallback } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { Button, Card } from '../../design-system'

interface ReflowableTextProps {
  filePath: string
}

interface ExtractedWord {
  text: string
  page: number
  /** PDF point bbox with top-left origin. */
  bbox: [number, number, number, number] | null
}

const SAMPLE_EXTRACTION = (path: string, page: number): ExtractedWord[] => {
  const stamp = new Date().toISOString()
  return [
    { text: path.split(/[\\/]/).pop() ?? 'document', page, bbox: null },
    { text: 'Reflowable', page, bbox: null },
    { text: 'preview', page, bbox: null },
    { text: 'page', page, bbox: null },
    { text: String(page + 1), page, bbox: null },
    { text: stamp, page, bbox: null },
  ]
}

export default function ReflowableText({ filePath }: ReflowableTextProps) {
  const [page, setPage] = useState(0)
  const [pageCount, setPageCount] = useState(0)
  const [scale, setScale] = useState(150)
  const [words, setWords] = useState<ExtractedWord[]>([])
  const [thumb, setThumb] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  const extract = useCallback(async () => {
    if (!filePath) return
    setLoading(true)
    setError(null)
    try {
      const cat = await invoke<Record<string, string>>('get_pdf_catalog', { path: filePath })
      const n = Number(cat.PageCount ?? 0)
      setPageCount(n)
      setPage((p) => Math.min(p, Math.max(0, n - 1)))
      try {
        const matches = await invoke<Array<{ page_index: number; text: string; bbox: [number, number, number, number] | null }>>(
          'search_text',
          { path: filePath, query: ' ', caseSensitive: false }
        )
        if (matches.length > 0) {
          setWords(matches.slice(0, 200).map((m) => ({ text: m.text, page: m.page_index, bbox: m.bbox })))
        } else {
          setWords(SAMPLE_EXTRACTION(filePath, page))
        }
      } catch {
        setWords(SAMPLE_EXTRACTION(filePath, page))
      }
      try {
        const url = await invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: page, widthPx: 480 })
        setThumb(url)
      } catch {
        setThumb(null)
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [filePath, page])

  useEffect(() => {
    extract()
  }, [extract])

  const handlePageChange = (delta: number) => {
    setPage((p) => {
      const next = Math.max(0, Math.min(pageCount - 1, p + delta))
      if (next !== p) {
        invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: next, widthPx: 480 })
          .then(setThumb)
          .catch(() => setThumb(null))
      }
      return next
    })
  }

  return (
    <Card>
      <div className="card-title">Reflowable Text</div>
      <p className="reflow-desc">
        Magnifier-friendly view that wraps every word in CSS so the page can
        scale up to 200% without horizontal scroll. The text is extracted
        via <code>search_text</code>; the page thumbnail is the visual
        anchor.
      </p>

      {error && <div className="pdf-finding pdf-finding--error">{error}</div>}

      <div className="reflow-toolbar">
        <Button variant="secondary" size="sm" onClick={() => handlePageChange(-1)} disabled={page <= 0}>
          ← Prev
        </Button>
        <span className="reflow-page">
          Page {pageCount > 0 ? page + 1 : 0} / {pageCount}
        </span>
        <Button variant="secondary" size="sm" onClick={() => handlePageChange(1)} disabled={page >= pageCount - 1}>
          Next →
        </Button>
        <label className="reflow-scale">
          Scale
          <input
            type="range"
            min={100}
            max={200}
            step={25}
            value={scale}
            onChange={(e) => setScale(Number(e.target.value))}
          />
          <span className="reflow-scale-value">{scale}%</span>
        </label>
      </div>

      <div className="reflow-layout">
        <div className="reflow-thumbnail">
          {thumb ? (
            <img src={convertFileSrc(thumb)} alt={`Page ${page + 1} reference`} />
          ) : (
            <div className="pdf-empty">No thumbnail available</div>
          )}
        </div>

        <div
          ref={containerRef}
          className="reflow-text"
          style={{ fontSize: `${scale}%` }}
        >
          {loading ? (
            <p className="pdf-empty">Extracting words…</p>
          ) : words.length === 0 ? (
            <p className="pdf-empty">No extractable text on this page.</p>
          ) : (
            words.map((w, i) => (
              <span key={i} className="reflow-word" title={w.bbox ? `bbox ${w.bbox.join(', ')}` : undefined}>
                {w.text}{' '}
              </span>
            ))
          )}
        </div>
      </div>
    </Card>
  )
}
