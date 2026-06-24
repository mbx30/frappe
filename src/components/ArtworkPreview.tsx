import { useEffect, useState, useMemo } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'

interface ArtworkPreviewProps {
  filePath: string
  onOpenInPdfTools?: (path: string) => void
  showOpenButton?: boolean
  height?: number
}

type FormatKind = 'pdf' | 'image' | 'tiff' | 'unsupported'

function classify(filePath: string): FormatKind {
  const ext = filePath.toLowerCase().split('.').pop() ?? ''
  if (ext === 'pdf') return 'pdf'
  if (['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'].includes(ext)) return 'image'
  if (['tif', 'tiff'].includes(ext)) return 'tiff'
  return 'unsupported'
}

interface PdfInfo {
  page_count: number
}

export default function ArtworkPreview({ filePath, onOpenInPdfTools, showOpenButton = true, height = 240 }: ArtworkPreviewProps) {
  const format = useMemo(() => classify(filePath), [filePath])
  const [error, setError] = useState<string | null>(null)
  const [pdfPageCount, setPdfPageCount] = useState<number | null>(null)
  const [pdfThumb, setPdfThumb] = useState<string | null>(null)
  const [expanded, setExpanded] = useState(false)
  const [fullThumb, setFullThumb] = useState<string | null>(null)

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => {
    setError(null)
    setPdfThumb(null)
    setPdfPageCount(null)
    setFullThumb(null)
  }, [filePath])

  useEffect(() => {
    if (format !== 'pdf') return

    let isMounted = true
    const fetchPdfInfo = async () => {
      try {
        const info = await invoke<PdfInfo>('open_pdf', { path: filePath, save: false }).catch(async () => {
          // open_pdf returns a PdfSummary; in case the call shape differs
          // across commits, fall back to using get_pdf_catalog which is
          // always present.
          const cat = await invoke<Record<string, string>>('get_pdf_catalog', { path: filePath })
          return { page_count: Number(cat.PageCount ?? 0) }
        })
        if (isMounted) setPdfPageCount(info.page_count)
        const thumb = await invoke<string>('render_page_thumbnail', {
          path: filePath,
          pageIndex: 0,
          widthPx: 320,
        })
        if (isMounted) setPdfThumb(thumb)
      } catch (e) {
        if (isMounted) setError(String(e))
      }
    }

    fetchPdfInfo()
    return () => {
      isMounted = false
    }
  }, [filePath, format])

  const handleExpand = async () => {
    if (format === 'pdf' && !fullThumb) {
      try {
        const url = await invoke<string>('render_page_thumbnail', {
          path: filePath,
          pageIndex: 0,
          widthPx: 1024,
        })
        setFullThumb(url)
      } catch (e) {
        setError(String(e))
        return
      }
    }
    setExpanded(true)
  }

  return (
    <div className="artwork-preview" style={{ minHeight: height }}>
      <div className="artwork-preview-frame" style={{ minHeight: height }}>
        {error && <div className="artwork-preview-error">{error}</div>}
        {format === 'pdf' && pdfThumb && (
          <img
            className="artwork-preview-img"
            src={convertFileSrc(pdfThumb)}
            alt={`PDF preview of ${filePath}`}
          />
        )}
        {format === 'pdf' && !pdfThumb && !error && (
          <div className="artwork-preview-loading">Rendering preview…</div>
        )}
        {format === 'image' && (
          <img
            className="artwork-preview-img"
            src={convertFileSrc(filePath)}
            alt={`Artwork preview of ${filePath}`}
          />
        )}
        {format === 'tiff' && (
          <div className="artwork-preview-fallback">
            <span className="artwork-preview-icon">🖼️</span>
            <p>TIFF preview not supported in the embedded viewer.</p>
            <p className="artwork-preview-hint">Open the file with the system image viewer to inspect it.</p>
          </div>
        )}
        {format === 'unsupported' && (
          <div className="artwork-preview-fallback">
            <span className="artwork-preview-icon">📄</span>
            <p>No inline preview available for this file type.</p>
          </div>
        )}
      </div>

      <div className="artwork-preview-meta">
        <span className="artwork-preview-name" title={filePath}>{filePath.split(/[\\/]/).pop()}</span>
        {format === 'pdf' && pdfPageCount !== null && (
          <span className="artwork-preview-pages">{pdfPageCount} page{pdfPageCount === 1 ? '' : 's'}</span>
        )}
        {format !== 'unsupported' && (
          <button className="artwork-preview-expand" onClick={handleExpand}>
            Expand
          </button>
        )}
        {showOpenButton && format === 'pdf' && onOpenInPdfTools && (
          <button className="artwork-preview-open" onClick={() => onOpenInPdfTools(filePath)}>
            Open in PDF Tools
          </button>
        )}
      </div>

      {expanded && (
        <div className="artwork-preview-modal" role="dialog" aria-label="Artwork preview">
          <div className="artwork-preview-modal-backdrop" onClick={() => setExpanded(false)} />
          <div className="artwork-preview-modal-content">
            <button className="artwork-preview-modal-close" onClick={() => setExpanded(false)} aria-label="Close preview">
              ✕
            </button>
            {format === 'pdf' && (fullThumb || pdfThumb) && (
              <img
                className="artwork-preview-modal-img"
                src={convertFileSrc(fullThumb ?? pdfThumb ?? '')}
                alt={`Large preview of ${filePath}`}
              />
            )}
            {format === 'image' && (
              <img
                className="artwork-preview-modal-img"
                src={convertFileSrc(filePath)}
                alt={`Large preview of ${filePath}`}
              />
            )}
            {format === 'tiff' && (
              <div className="artwork-preview-fallback">
                <span className="artwork-preview-icon">🖼️</span>
                <p>TIFF preview not supported in the embedded viewer.</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
