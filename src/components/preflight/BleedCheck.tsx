import { useState, useRef, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { BleedFinding } from '../../types'

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
      setFixResult(`Bleed added — saved as ${outPath}`)
    } catch (e) {
      setFixResult(`Error: ${e}`)
    } finally {
      setFixing(false)
    }
  }

  return (
    <div className="pdf-preflight-section">
      <div className="pdf-preflight-header">
        <h4>Bleed Check</h4>
        <label className="dpi-slider">
          Min bleed: {minBleedMm}mm
          <input type="range" min="1" max="10" step="0.5" value={minBleedMm}
            onChange={e => handleSliderChange(Number(e.target.value))} />
        </label>
      </div>
      {findings.length === 0 ? (
        <p className="pdf-empty">No bleed data available.</p>
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
                <span className="pdf-finding-message">No BleedBox</span>
              )}
            </div>
          ))}
        </div>
      )}
      {(hasErrors || findings.some(f => f.severity === 'warning')) && (
        <div className="bleed-fixup">
          <div className="bleed-fixup-row">
            <label>Add bleed:</label>
            <input type="number" min="1" max="20" step="0.5" value={fixAmount}
              onChange={e => setFixAmount(Number(e.target.value))} />
            <span>mm</span>
            <button className="btn btn-secondary" onClick={handleAddBleed} disabled={fixing}>
              {fixing ? 'Adding...' : 'Add Bleed & Save'}
            </button>
          </div>
          {fixResult && <p className="bleed-result">{fixResult}</p>}
        </div>
      )}
    </div>
  )
}
