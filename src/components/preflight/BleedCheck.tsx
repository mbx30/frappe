import { useState, useRef, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { BleedFinding } from '../../types'
import { t } from '../../i18n'

interface BleedCheckProps {
  filePath: string
  findings: BleedFinding[]
  minBleedMm: number
  onMinBleedChange: (mm: number) => void
  onRerun: (minMm: number) => Promise<void>
}

function formatMm(v: number): string {
  return v.toFixed(1) + ' mm'
}

export default function BleedCheck({ filePath, findings, minBleedMm, onMinBleedChange, onRerun }: BleedCheckProps) {
  const [fixAmount, setFixAmount] = useState(3)
  const [fixing, setFixing] = useState(false)
  const [fixResult, setFixResult] = useState<string | null>(null)
  const debounceTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const handleSliderChange = (value: number) => {
    onMinBleedChange(value)
    if (debounceTimeoutRef.current) clearTimeout(debounceTimeoutRef.current)
    debounceTimeoutRef.current = setTimeout(() => {
      onRerun(value)
    }, 300)
  }

  useEffect(() => {
    return () => {
      if (debounceTimeoutRef.current) clearTimeout(debounceTimeoutRef.current)
    }
  }, [])

  const hasErrors = findings.some(f => f.severity === 'error')

  const handleAddBleed = async () => {
    setFixing(true)
    setFixResult(null)
    try {
      const outPath = filePath.replace(/\.pdf$/i, `_bleed${fixAmount}mm.pdf`)
      await invoke('add_bleed', { path: filePath, amountMm: fixAmount, outputPath: outPath })
      setFixResult(t('bleed.check.added', { path: outPath }))
    } catch (e) {
      setFixResult(t('bleed.check.error', { msg: String(e) }))
    } finally {
      setFixing(false)
    }
  }

  return (
    <div className="pdf-preflight-section">
      <div className="pdf-preflight-header">
        <h4>{t('bleed.check.title')}</h4>
        <label className="dpi-slider">
          {t('bleed.check.min_bleed', { n: minBleedMm })}
          <input type="range" min="1" max="10" step="0.5" value={minBleedMm}
            onChange={e => handleSliderChange(Number(e.target.value))} />
        </label>
      </div>
      {findings.length === 0 ? (
        <p className="pdf-empty">{t('bleed.check.no_data')}</p>
      ) : (
        <div className="bleed-table">
          {findings.map((f, i) => (
            <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
              <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
              <span className="pdf-finding-name">P.{f.page}</span>
              {f.has_bleed_box ? (
                <span className="pdf-finding-message">
                  T:{formatMm(f.bleed_top_mm)} R:{formatMm(f.bleed_right_mm)}
                  {' '}B:{formatMm(f.bleed_bottom_mm)} L:{formatMm(f.bleed_left_mm)}
                </span>
              ) : (
                <span className="pdf-finding-message">{t('bleed.check.no_bleed_box')}</span>
              )}
            </div>
          ))}
        </div>
      )}
      {(hasErrors || findings.some(f => f.severity === 'warning')) && (
        <div className="bleed-fixup">
          <div className="bleed-fixup-row">
            <label>{t('bleed.check.add')}</label>
            <input type="number" min="1" max="20" step="0.5" value={fixAmount}
              onChange={e => setFixAmount(Number(e.target.value))} />
            <span>{t('bleed.check.unit')}</span>
            <button className="btn btn-secondary" onClick={handleAddBleed} disabled={fixing}>
              {fixing ? t('bleed.check.adding') : t('bleed.check.add_save')}
            </button>
          </div>
          {fixResult && <p className="bleed-result">{fixResult}</p>}
        </div>
      )}
    </div>
  )
}
