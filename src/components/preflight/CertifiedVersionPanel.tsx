import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { CertifiedVersion } from '../../types'

interface CertifiedVersionPanelProps {
  jobId: number | null
  filePath: string
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export default function CertifiedVersionPanel({ jobId, filePath }: CertifiedVersionPanelProps) {
  const [versions, setVersions] = useState<CertifiedVersion[]>([])
  const [author, setAuthor] = useState('')
  const [comment, setComment] = useState('')
  const [saving, setSaving] = useState(false)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (jobId) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setLoading(true)
      invoke<CertifiedVersion[]>('list_certified_versions', { jobId })
        .then(setVersions)
        .catch(() => {})
        .finally(() => setLoading(false))
    }
  }, [jobId])

  const handleSaveVersion = async () => {
    if (!jobId || !author.trim()) return
    setSaving(true)
    try {
      await invoke('create_certified_version', { jobId, filePath, author: author.trim(), comment: comment.trim() })
      const updated = await invoke<CertifiedVersion[]>('list_certified_versions', { jobId })
      setVersions(updated)
      setComment('')
    } catch (e) {
      console.error('Failed to save version:', e)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="certified-version-panel">
      <div className="pdf-preflight-header">
        <h4>Certified PDF — Version Tracking</h4>
      </div>

      {!jobId ? (
        <p className="pdf-empty">Save the PDF to history first to enable version tracking.</p>
      ) : (
        <>
          <div className="certified-form">
            <div className="conversion-field">
              <label className="pdf-label">Author</label>
              <input
                type="text"
                className="form-input"
                value={author}
                onChange={e => setAuthor(e.target.value)}
                placeholder="Your name"
              />
            </div>
            <div className="conversion-field">
              <label className="pdf-label">Version Comment</label>
              <textarea
                className="form-input"
                value={comment}
                onChange={e => setComment(e.target.value)}
                placeholder="What changed in this version?"
                rows={2}
              />
            </div>
            <button className="btn btn-primary" onClick={handleSaveVersion} disabled={saving || !author.trim()}>
              {saving ? 'Saving...' : 'Save Certified Version'}
            </button>
          </div>

          <div className="certified-versions">
            <h5>Version History</h5>
            {loading && <p className="pdf-empty">Loading...</p>}
            {!loading && versions.length === 0 && (
              <p className="pdf-empty">No versions saved yet.</p>
            )}
            {versions.map((v, i) => (
              <div key={v.id} className={`certified-version-item ${i === 0 ? 'certified-version-item--latest' : ''}`}>
                <div className="certified-version-header">
                  <span className="certified-version-badge">v{v.version_number}</span>
                  {i === 0 && <span className="certified-version-current">current</span>}
                  {v.is_signed ? <span className="certified-version-signed">signed</span> : null}
                </div>
                <div className="certified-version-info">
                  <span className="certified-version-author">{v.author}</span>
                  <span className="certified-version-date">{v.created_at}</span>
                  <span className="certified-version-size">{formatBytes(v.file_size_bytes)}</span>
                </div>
                {v.comment && <p className="certified-version-comment">{v.comment}</p>}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
