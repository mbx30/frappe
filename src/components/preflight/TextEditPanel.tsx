import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { TextMatch } from '../../types'

interface TextEditPanelProps {
  filePath?: string
  pageCount?: number
}

export default function TextEditPanel({ filePath, pageCount }: TextEditPanelProps) {
  const [query, setQuery] = useState('')
  const [replace, setReplace] = useState('')
  const [matches, setMatches] = useState<TextMatch[]>([])
  const [currentIndex, setCurrentIndex] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [replacementPath, setReplacementPath] = useState<string | null>(null)

  const runSearch = useCallback(async () => {
    if (!filePath || !query) return
    setBusy(true)
    setError(null)
    try {
      const result = await invoke<TextMatch[]>('search_text', {
        path: filePath,
        query,
      })
      setMatches(result)
      setCurrentIndex(0)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }, [filePath, query])

  const runReplace = useCallback(async () => {
    if (!filePath || !query) return
    if (matches.length === 0) {
      setError('Run a search first.')
      return
    }
    // The replace_text command takes a single page index; we
    // replace on the page that contains the current match.
    const target = matches[currentIndex]
    if (!target) return
    setBusy(true)
    setError(null)
    try {
      const out = filePath.replace(/\.pdf$/i, '_replaced.pdf')
      const res = await invoke<{ replacements_made: number; output_path: string }>(
        'replace_text',
        {
          path: filePath,
          pageIndex: target.page_index,
          find: query,
          replace,
          outputPath: out,
        }
      )
      setReplacementPath(res.output_path)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }, [filePath, query, replace, matches, currentIndex])

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => {
    setMatches([])
    setCurrentIndex(0)
    setReplacementPath(null)
  }, [filePath, query])

  if (!filePath) {
    return (
      <div className="text-edit-panel">
        <h3>Text Edit</h3>
        <p className="pdf-empty">Open a PDF to search and replace text.</p>
      </div>
    )
  }

  return (
    <div className="text-edit-panel">
      <h3>Text Edit</h3>
      {pageCount && <p className="text-edit-count">{pageCount} pages</p>}
      {error && <p className="pdf-error">{error}</p>}
      {replacementPath && (
        <p className="text-edit-ok">Replaced output: <code>{replacementPath}</code></p>
      )}

      <div className="text-edit-row">
        <label>
          Find
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="text to find"
          />
        </label>
        <label>
          Replace
          <input
            type="text"
            value={replace}
            onChange={(e) => setReplace(e.target.value)}
            placeholder="replacement (empty = delete)"
          />
        </label>
      </div>

      <div className="text-edit-buttons">
        <button
          className="btn btn-secondary"
          onClick={runSearch}
          disabled={busy || !query}
        >
          {busy && matches.length === 0 ? 'Searching...' : 'Search'}
        </button>
        <button
          className="btn btn-primary"
          onClick={runReplace}
          disabled={busy || !query || matches.length === 0}
        >
          {busy && replacementPath === null ? 'Replacing...' : 'Replace'}
        </button>
      </div>

      {matches.length > 0 && (
        <div className="text-edit-matches">
          <p className="text-edit-count">
            {matches.length} match{matches.length === 1 ? '' : 'es'} found
          </p>
          <ul>
            {matches.map((m, i) => (
              <li
                key={i}
                className={i === currentIndex ? 'text-edit-match active' : 'text-edit-match'}
                onClick={() => setCurrentIndex(i)}
              >
                <span className="text-edit-match-page">P.{m.page_index + 1}</span>
                <span className="text-edit-match-text">{m.text}</span>
                {m.bbox && (
                  <span className="text-edit-match-bbox">
                    [{m.bbox[0].toFixed(1)}, {m.bbox[1].toFixed(1)} -
                    {m.bbox[2].toFixed(1)}, {m.bbox[3].toFixed(1)}]
                  </span>
                )}
              </li>
            ))}
          </ul>
          <div className="text-edit-nav">
            <button
              className="btn btn-secondary btn-small"
              disabled={currentIndex === 0}
              onClick={() => setCurrentIndex((i) => Math.max(0, i - 1))}
            >
              Prev
            </button>
            <span>{currentIndex + 1} / {matches.length}</span>
            <button
              className="btn btn-secondary btn-small"
              disabled={currentIndex >= matches.length - 1}
              onClick={() => setCurrentIndex((i) => Math.min(matches.length - 1, i + 1))}
            >
              Next
            </button>
          </div>
        </div>
      )}

      {matches.length === 0 && query && !busy && (
        <p className="text-edit-empty">No matches.</p>
      )}
    </div>
  )
}
