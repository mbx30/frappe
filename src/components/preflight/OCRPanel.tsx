import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PdfType, OcrResult, OcrBackend, CostEstimate } from '../../types'
import { t } from '../../i18n'

interface OCRPanelProps {
  filePath: string
  pageCount: number
}

export default function OCRPanel({ filePath, pageCount }: OCRPanelProps) {
  // PDF type detection
  const [pdfType, setPdfType] = useState<PdfType | null>(null)
  const [detectingType, setDetectingType] = useState(false)

  // OCR options
  const [backend, setBackend] = useState<OcrBackend>('Tesseract')
  const [language, setLanguage] = useState('eng')
  const [overlayText, setOverlayText] = useState(true)
  const [selectedPages, setSelectedPages] = useState<number[]>([])
  const [useAllPages, setUseAllPages] = useState(true)

  // API key management
  const [testingConnection, setTestingConnection] = useState(false)
  const [connectionValid, setConnectionValid] = useState<boolean | null>(null)

  // Cost estimation (for Google Vision)
  const [costEstimate, setCostEstimate] = useState<CostEstimate | null>(null)

  // OCR execution
  const [running, setRunning] = useState(false)
  const [progress, setProgress] = useState(0)
  const [result, setResult] = useState<OcrResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Detect PDF type on load
  useEffect(() => {
    const detectType = async () => {
      setDetectingType(true)
      try {
        const type = await invoke<PdfType>('detect_pdf_type', { path: filePath })
        setPdfType(type)
      } catch (e) {
        console.error('Failed to detect PDF type:', e)
      } finally {
        setDetectingType(false)
      }
    }
    detectType()
  }, [filePath])

  // Estimate cost when backend changes to Google Vision
  useEffect(() => {
    if (backend === 'GoogleCloudVision') {
      const estimateCost = async () => {
        try {
          const estimate = await invoke<CostEstimate>('estimate_google_vision_cost', { path: filePath })
          setCostEstimate(estimate)
        } catch (e) {
          console.error('Failed to estimate cost:', e)
        }
      }
      estimateCost()
    }
  }, [backend, filePath])

  const testConnection = async () => {
    setTestingConnection(true)
    try {
      const valid = await invoke<boolean>('test_google_vision_connection')
      setConnectionValid(valid)
    } catch (e) {
      console.error('Connection test failed:', e)
      setConnectionValid(false)
    } finally {
      setTestingConnection(false)
    }
  }

  const handleRunOCR = async () => {
    setRunning(true)
    setError(null)
    setResult(null)
    setProgress(0)

    try {
      const pagesToProcess = useAllPages ? [] : selectedPages
      const outputPath = overlayText
        ? filePath.replace(/\.pdf$/i, '_OCR.pdf')
        : null

      const result = await invoke<OcrResult>('run_ocr', {
        path: filePath,
        options: {
          pages: pagesToProcess,
          backend,
          overlay_text: overlayText,
          output_path: outputPath,
          language,
        },
      })

      setResult(result)
      setProgress(100)
    } catch (e) {
      setError(String(e))
    } finally {
      setRunning(false)
    }
  }

  const getPdfTypeLabel = (type: PdfType): string => {
    if (type === 'TextBased') {
      return 'Text-Based (Already Searchable)'
    } else if (type === 'Scanned') {
      return 'Scanned (Requires OCR)'
    } else if (typeof type === 'object' && 'Mixed' in type) {
      const mixed = type.Mixed
      return `Mixed (${mixed.text_pages.length} text, ${mixed.scanned_pages.length} scanned)`
    }
    return 'Unknown'
  }

  const needsApiKey = backend === 'GoogleCloudVision'

  return (
    <div className="ocr-panel">
      <div className="pdf-preflight-header">
        <h4>{t('ocr.title')}</h4>
        {pdfType === 'Scanned' && (
          <span className="pdf-badge pdf-badge-warning">{t('ocr.scanned_detected')}</span>
        )}
        {pdfType === 'TextBased' && (
          <span className="pdf-badge pdf-badge-success">{t('ocr.already_searchable')}</span>
        )}
      </div>

      {detectingType && <p className="pdf-empty">{t('ocr.detecting_type')}</p>}

      {!detectingType && pdfType && (
        <>
          {/* PDF Type Detection Result */}
          <div className="ocr-type-info">
            <p><strong>{t('ocr.pdf_type')}:</strong> {getPdfTypeLabel(pdfType)}</p>
            {pdfType === 'TextBased' && (
              <p className="pdf-empty">{t('ocr.already_searchable_desc')}</p>
            )}
          </div>

          {/* OCR Options */}
          <div className="ocr-options">
            <div className="ocr-field">
              <label className="pdf-label">{t('ocr.backend')}</label>
              <select
                value={backend}
                onChange={(e) => setBackend(e.target.value as OcrBackend)}
                disabled={running}
                className="form-select"
              >
                <option value="Tesseract">Local Tesseract</option>
                <option value="GoogleCloudVision">Google Cloud Vision</option>
              </select>
            </div>

            <div className="ocr-field">
              <label className="pdf-label">{t('ocr.language')}</label>
              <select
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
                disabled={running}
                className="form-select"
              >
                <option value="eng">English</option>
                <option value="fra">French</option>
                <option value="deu">German</option>
                <option value="spa">Spanish</option>
                <option value="ita">Italian</option>
                <option value="nld">Dutch</option>
                <option value="por">Portuguese</option>
                <option value="jpn">Japanese</option>
                <option value="zho">Chinese</option>
              </select>
            </div>

            {/* Page Selection */}
            <div className="ocr-field">
              <label className="pdf-checkbox">
                <input
                  type="checkbox"
                  checked={useAllPages}
                  onChange={(e) => setUseAllPages(e.target.checked)}
                  disabled={running}
                />
                {t('ocr.all_pages')}
              </label>
            </div>

            {!useAllPages && (
              <div className="ocr-field">
                <label className="pdf-label">{t('ocr.selected_pages')}</label>
                <input
                  type="text"
                  placeholder={t('ocr.pages_placeholder')}
                  value={selectedPages.join(',')}
                  onChange={(e) => {
                    const pages = e.target.value
                      .split(',')
                      .map((p) => parseInt(p.trim(), 10))
                      .filter((p) => !isNaN(p) && p >= 0 && p < pageCount)
                    setSelectedPages(pages)
                  }}
                  disabled={running}
                  className="form-input"
                />
              </div>
            )}

            {/* Overlay Text Option */}
            <div className="ocr-field">
              <label className="pdf-checkbox">
                <input
                  type="checkbox"
                  checked={overlayText}
                  onChange={(e) => setOverlayText(e.target.checked)}
                  disabled={running}
                />
                {t('ocr.overlay_text')}
              </label>
            </div>

            {/* Google Cloud Vision API Key Setup */}
            {needsApiKey && (
              <div className="ocr-api-key-section">
                <div className="pdf-preflight-header">
                  <h5>{t('ocr.api_key_title')}</h5>
                  {connectionValid === true && (
                    <span className="pdf-badge pdf-badge-success">{t('ocr.api_connected')}</span>
                  )}
                  {connectionValid === false && (
                    <span className="pdf-badge pdf-badge-error">{t('ocr.api_error')}</span>
                  )}
                </div>

                <div className="ocr-field">
                  <input
                    type="password"
                    placeholder={t('ocr.api_key_placeholder')}
                    className="form-input"
                    disabled={running || testingConnection}
                  />
                </div>

                <button
                  className="btn btn-secondary"
                  onClick={testConnection}
                  disabled={running || testingConnection}
                >
                  {testingConnection ? t('ocr.testing') : t('ocr.test_connection')}
                </button>

                {costEstimate && (
                  <div className="ocr-cost-estimate">
                    <p>
                      {t('ocr.cost_estimate')}:{' '}
                      <strong>${costEstimate.cost_usd.toFixed(2)}</strong> USD
                    </p>
                    <p className="cost-detail">
                      {t('ocr.cost_detail', {
                        pages: costEstimate.page_count,
                        billable: costEstimate.billable_pages,
                      })}
                    </p>
                  </div>
                )}
              </div>
            )}

            {/* Run OCR Button */}
            <button
              className="btn btn-primary"
              onClick={handleRunOCR}
              disabled={running || (needsApiKey && connectionValid !== true)}
            >
              {running ? `${t('ocr.running')} ${progress}%` : t('ocr.run')}
            </button>

            {/* Progress Bar */}
            {running && (
              <div className="ocr-progress">
                <div className="progress-bar" style={{ width: `${progress}%` }} />
              </div>
            )}

            {/* Error Message */}
            {error && <div className="pdf-finding pdf-finding--error">{error}</div>}

            {/* Results */}
            {result && (
              <div className="ocr-results">
                <div className="pdf-preflight-header">
                  <h5>{t('ocr.results_title')}</h5>
                </div>

                <div className="ocr-result-summary">
                  <p>
                    <strong>{t('ocr.pages_processed')}:</strong> {result.pages_processed}
                  </p>
                  <p>
                    <strong>{t('ocr.backend_used')}:</strong> {result.backend}
                  </p>
                  <p>
                    <strong>{t('ocr.time_taken')}:</strong> {(result.duration_ms / 1000).toFixed(2)}s
                  </p>
                </div>

                {/* Per-page Results */}
                <div className="ocr-pages-list">
                  {result.pages.map((page) => (
                    <div key={page.page_index} className="ocr-page-result">
                      <div className="page-header">
                        <span>{t('ocr.page')} {page.page_index + 1}</span>
                        <span className="confidence-badge">
                          {Math.round(page.confidence * 100)}% {t('ocr.confidence')}
                        </span>
                      </div>
                      {page.text && (
                        <p className="page-text">{page.text.substring(0, 200)}...</p>
                      )}
                    </div>
                  ))}
                </div>

                {overlayText && (
                  <p className="pdf-empty">
                    {t('ocr.saved_as', { path: filePath.replace(/\.pdf$/i, '_OCR.pdf') })}
                  </p>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  )
}
