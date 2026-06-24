import { useState, useEffect } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { Button, Card, Input } from '../../design-system'

interface AltTextEditorProps {
  filePath: string
  /** Optional page index to render in the preview pane. */
  pageIndex?: number
}

interface ImageEntry {
  object_id: number
  page: number
  bbox: [number, number, number, number] | null
  alt_text: string
  is_decorative: boolean
  saved: boolean
}

const PALETTE = ['#2563eb', '#16a34a', '#dc2626', '#f59e0b', '#9333ea', '#0ea5e9']

function colorFor(id: number) {
  return PALETTE[Math.abs(id) % PALETTE.length]
}

export default function AltTextEditor({ filePath, pageIndex = 0 }: AltTextEditorProps) {
  const [images, setImages] = useState<ImageEntry[]>([])
  const [thumb, setThumb] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [savedMessage, setSavedMessage] = useState<string | null>(null)
  const [page, setPage] = useState(pageIndex)

  useEffect(() => {
    if (!filePath) return

    let isMounted = true
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const stored = await invoke<Array<[number, string, boolean]>>('list_alt_text', { filePath })
        const storedMap = new Map<number, { alt_text: string; is_decorative: boolean }>()
        for (const [oid, alt, deco] of stored) {
          storedMap.set(oid, { alt_text: alt, is_decorative: deco })
        }

        try {
          const cat = await invoke<Record<string, string>>('get_pdf_catalog', { path: filePath })
          const pageCount = Number(cat.PageCount ?? 0)
          const imageCount = Number(cat.ImageCount ?? 0)
          const entries: ImageEntry[] = []
          for (let i = 0; i < Math.min(imageCount, 50); i += 1) {
            const oid = i + 1
            const storedEntry = storedMap.get(oid)
            entries.push({
              object_id: oid,
              page: 0,
              bbox: null,
              alt_text: storedEntry?.alt_text ?? '',
              is_decorative: storedEntry?.is_decorative ?? false,
              saved: !!storedEntry,
            })
          }
          if (isMounted) {
            setImages(entries)
            setPage((p) => Math.min(p, Math.max(0, pageCount - 1)))
          }
        } catch (e) {
          if (isMounted) setError(String(e))
        }

        try {
          const url = await invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: page, widthPx: 480 })
          if (isMounted) setThumb(url)
        } catch {
          if (isMounted) setThumb(null)
        }
      } catch (e) {
        if (isMounted) setError(String(e))
      } finally {
        if (isMounted) setLoading(false)
      }
    }

    load()
    return () => {
      isMounted = false
    }
  }, [filePath, page])

  const updateField = (id: number, patch: Partial<ImageEntry>) => {
    setImages((prev) => prev.map((e) => (e.object_id === id ? { ...e, ...patch, saved: false } : e)))
  }

  const save = async (entry: ImageEntry) => {
    try {
      await invoke('set_alt_text', {
        filePath,
        objectId: entry.object_id,
        altText: entry.alt_text,
        isDecorative: entry.is_decorative,
      })
      setImages((prev) => prev.map((e) => (e.object_id === entry.object_id ? { ...e, saved: true } : e)))
      setSavedMessage(`Saved alt text for image #${entry.object_id}.`)
      setTimeout(() => setSavedMessage(null), 2500)
    } catch (e) {
      setError(String(e))
    }
  }

  const saveAll = async () => {
    for (const entry of images.filter((e) => !e.saved)) {
      await save(entry)
    }
  }

  return (
    <Card>
      <div className="card-title">Alt Text Editor</div>
      <p className="alt-editor-desc">
        Right-click an image on the PDF to add alt text. (This panel is the
        bulk-edit equivalent — the right-click hookup is wired in the PDF
        viewer and writes back through the same <code>set_alt_text</code>
        command.)
      </p>

      {error && <div className="pdf-finding pdf-finding--error">{error}</div>}
      {savedMessage && <div className="pdf-finding pdf-finding--ok" role="status">{savedMessage}</div>}

      <div className="alt-editor-layout">
        <div className="alt-editor-preview">
          {thumb ? (
            <img src={convertFileSrc(thumb)} alt={`Page ${page + 1} preview`} />
          ) : (
            <div className="pdf-empty">No thumbnail available</div>
          )}
          <p className="alt-editor-page">Page {page + 1}</p>
        </div>

        <div className="alt-editor-list">
          <div className="alt-editor-actions">
            <Button variant="primary" size="sm" onClick={saveAll} disabled={loading || images.length === 0}>
              Save all
            </Button>
          </div>
          {loading ? (
            <p className="pdf-empty">Loading images…</p>
          ) : images.length === 0 ? (
            <p className="pdf-empty">No images detected on this document.</p>
          ) : (
            <ul className="alt-editor-entries">
              {images.map((entry) => (
                <li key={entry.object_id} className="alt-editor-entry">
                  <div className="alt-editor-entry-head">
                    <span
                      className="alt-editor-marker"
                      style={{ background: colorFor(entry.object_id) }}
                      aria-hidden="true"
                    />
                    <span className="alt-editor-id">Image #{entry.object_id}</span>
                    <label className="alt-editor-decorative">
                      <input
                        type="checkbox"
                        checked={entry.is_decorative}
                        onChange={(e) => updateField(entry.object_id, { is_decorative: e.target.checked })}
                      />
                      Decorative
                    </label>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => save(entry)}
                      disabled={entry.saved}
                    >
                      {entry.saved ? 'Saved' : 'Save'}
                    </Button>
                  </div>
                  <Input
                    placeholder="Describe the image for screen readers…"
                    value={entry.alt_text}
                    onChange={(e) => updateField(entry.object_id, { alt_text: e.target.value })}
                    disabled={entry.is_decorative}
                  />
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </Card>
  )
}
