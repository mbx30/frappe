import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { t } from '../../i18n'

interface PdfInspectorProps {
  filePath: string
}

export default function PdfInspector({ filePath }: PdfInspectorProps) {
  const [catalog, setCatalog] = useState<Record<string, string> | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    /* eslint-disable react-hooks/set-state-in-effect */
    setLoading(true)
    setError(null)
    /* eslint-enable react-hooks/set-state-in-effect */
    invoke<Record<string, string>>('get_pdf_catalog', { path: filePath })
      .then((data) => {
        const clean: Record<string, string> = {}
        for (const [k, v] of Object.entries(data)) {
          if (typeof v === 'object') {
            clean[k] = JSON.stringify(v)
          } else {
            clean[k] = String(v)
          }
        }
        setCatalog(clean)
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false))
  }, [filePath])

  if (loading) return <div className="pdf-empty">{t('inspector.loading')}</div>
  if (error) return <div className="pdf-finding pdf-finding--error">{t('inspector.error', { msg: error })}</div>
  if (!catalog) return null

  const excludedKeys = new Set(['PageCount', 'PDFVersion'])

  return (
    <div className="pdf-preflight-section">
      <h4>{t('inspector.pdf.title')}</h4>
      <div className="inspector-grid">
        <div className="inspector-item">
          <span className="pdf-label">{t('inspector.pages')}</span>
          <span className="pdf-value">{catalog.PageCount ?? '?'}</span>
        </div>
        <div className="inspector-item">
          <span className="pdf-label">{t('inspector.pdf_version')}</span>
          <span className="pdf-value">{catalog.PDFVersion ?? '?'}</span>
        </div>
      </div>
      <h4 style={{ marginTop: 12 }}>{t('inspector.catalog')}</h4>
      <div className="inspector-entries">
        {Object.entries(catalog).map(([key, value]) => {
          if (excludedKeys.has(key)) return null
          return (
            <div key={key} className="inspector-row">
              <span className="inspector-key">/{key}</span>
              <span className="inspector-value">{value}</span>
            </div>
          )
        })}
      </div>
    </div>
  )
}
