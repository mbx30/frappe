import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { OptimizeSettings } from '../../types'

interface ImageEditPanelProps {
  filePath?: string
  pageIndex?: number
}

function deriveOutPath(filePath: string, suffix: string): string {
  return filePath.replace(/\.pdf$/i, `_${suffix}.pdf`)
}

export default function ImageEditPanel({ filePath, pageIndex = 0 }: ImageEditPanelProps) {
  const [busy, setBusy] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [xobjectName, setXobjectName] = useState('')
  const [newImagePath, setNewImagePath] = useState('')
  const [maxWidth, setMaxWidth] = useState(0)
  const [maxHeight, setMaxHeight] = useState(0)
  const [quality, setQuality] = useState(85)
  const [convertToJpeg, setConvertToJpeg] = useState(true)

  const runReplace = useCallback(async () => {
    if (!filePath || !newImagePath) return
    setBusy('replace')
    setError(null)
    setMessage(null)
    try {
      const out = deriveOutPath(filePath, 'replaced')
      await invoke('replace_image', {
        path: filePath,
        pageIndex,
        xobjectName,
        newImagePath,
        outputPath: out,
      })
      setMessage(`Replaced -> ${out}`)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(null)
    }
  }, [filePath, pageIndex, xobjectName, newImagePath])

  const runOptimize = useCallback(async () => {
    if (!filePath) return
    setBusy('optimize')
    setError(null)
    setMessage(null)
    try {
      const out = deriveOutPath(filePath, 'optimized')
      const settings: OptimizeSettings = {
        max_width: maxWidth > 0 ? maxWidth : undefined,
        max_height: maxHeight > 0 ? maxHeight : undefined,
        quality,
        convert_to_jpeg: convertToJpeg,
      }
      await invoke('optimize_image', {
        path: filePath,
        pageIndex,
        xobjectName,
        settings,
        outputPath: out,
      })
      setMessage(`Optimized -> ${out}`)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(null)
    }
  }, [filePath, pageIndex, xobjectName, maxWidth, maxHeight, quality, convertToJpeg])

  if (!filePath) {
    return (
      <div className="image-edit-panel">
        <h3>Image Editor</h3>
        <p className="pdf-empty">Open a PDF to edit its images.</p>
      </div>
    )
  }

  return (
    <div className="image-edit-panel">
      <h3>Image Editor</h3>
      <p className="image-edit-help">
        Replace or optimize the first image on page {pageIndex + 1}. Leave
        the XObject name empty to target the first image; specify a name to
        target a specific one.
      </p>
      {error && <p className="pdf-error">{error}</p>}
      {message && <p className="image-edit-ok">{message}</p>}

      <section className="image-edit-section">
        <label>
          XObject name
          <input
            type="text"
            value={xobjectName}
            onChange={(e) => setXobjectName(e.target.value)}
            placeholder="(first image)"
          />
        </label>

        <label>
          Replacement image path
          <input
            type="text"
            value={newImagePath}
            onChange={(e) => setNewImagePath(e.target.value)}
            placeholder="C:/path/to/new.png"
          />
        </label>
        <button
          className="btn btn-primary"
          disabled={busy !== null || !newImagePath}
          onClick={runReplace}
        >
          {busy === 'replace' ? 'Replacing...' : 'Replace Image'}
        </button>
      </section>

      <section className="image-edit-section">
        <h4>Optimize</h4>
        <label>
          Max width (px)
          <input
            type="number"
            value={maxWidth}
            onChange={(e) => setMaxWidth(Number(e.target.value))}
            min={0}
          />
        </label>
        <label>
          Max height (px)
          <input
            type="number"
            value={maxHeight}
            onChange={(e) => setMaxHeight(Number(e.target.value))}
            min={0}
          />
        </label>
        <label>
          JPEG quality
          <input
            type="range"
            min={1}
            max={100}
            value={quality}
            onChange={(e) => setQuality(Number(e.target.value))}
          />
          <span className="image-edit-quality-value">{quality}</span>
        </label>
        <label className="image-edit-checkbox">
          <input
            type="checkbox"
            checked={convertToJpeg}
            onChange={(e) => setConvertToJpeg(e.target.checked)}
          />
          Convert to JPEG (uncheck to keep PNG grayscale for ink-saver)
        </label>
        <button
          className="btn btn-secondary"
          disabled={busy !== null}
          onClick={runOptimize}
        >
          {busy === 'optimize' ? 'Optimizing...' : 'Optimize Image'}
        </button>
      </section>
    </div>
  )
}
