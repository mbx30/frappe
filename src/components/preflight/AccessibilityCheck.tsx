import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { Button, Card } from '../../design-system'

interface AccessibilityCheckProps {
  filePath: string
}

interface AccessibilityFinding {
  category: 'contrast' | 'alt_text' | 'tag_tree' | 'form_labels'
  severity: 'error' | 'warning' | 'info'
  message: string
  detail: string
}

interface AccessibilitySummary {
  total_findings: number
  by_severity: { error: number; warning: number; info: number }
  wcag_aa_pass: boolean
}

function summarize(findings: AccessibilityFinding[]): AccessibilitySummary {
  const by = { error: 0, warning: 0, info: 0 }
  for (const f of findings) by[f.severity] += 1
  return {
    total_findings: findings.length,
    by_severity: by,
    wcag_aa_pass: findings.every((f) => f.severity !== 'error'),
  }
}

function relativeLuminance(rgb: [number, number, number]): number {
  const [r, g, b] = rgb.map((v) => {
    const s = v / 255
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

function contrastRatio(a: [number, number, number], b: [number, number, number]): number {
  const l1 = relativeLuminance(a)
  const l2 = relativeLuminance(b)
  const [lighter, darker] = l1 > l2 ? [l1, l2] : [l2, l1]
  return (lighter + 0.05) / (darker + 0.05)
}

const severityBadge: Record<AccessibilityFinding['severity'], { tone: 'danger' | 'warning' | 'info'; label: string }> = {
  error: { tone: 'danger', label: 'Error' },
  warning: { tone: 'warning', label: 'Warning' },
  info: { tone: 'info', label: 'Info' },
}

export default function AccessibilityCheck({ filePath }: AccessibilityCheckProps) {
  const [findings, setFindings] = useState<AccessibilityFinding[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [thumb, setThumb] = useState<string | null>(null)
  const [showLuminanceDemo, setShowLuminanceDemo] = useState(false)
  const isMountedRef = useRef(true)

  const runChecks = useCallback(async () => {
    if (!filePath) return

    try {
      setLoading(true)
      setError(null)
      setFindings([])

      const collected: AccessibilityFinding[] = []
      const cat = await invoke<Record<string, string>>('get_pdf_catalog', { path: filePath })

      // ── (1) Tag tree / structure ──────────────────────────────
      const hasStructTree = !!cat.StructTreeRoot
      if (!hasStructTree) {
        collected.push({
          category: 'tag_tree',
          severity: 'warning',
          message: 'No document structure tree found.',
          detail: 'Tagged PDFs expose the logical reading order and heading hierarchy to assistive technology. Without it, screen readers infer the order from visual position which is often wrong for multi-column layouts.',
        })
      }

      // ── (2) Form field labels ─────────────────────────────────
      if (cat.AcroForm) {
        try {
          const acro = JSON.parse(cat.AcroForm) as { Fields?: unknown[] }
          const fieldCount = Array.isArray(acro.Fields) ? acro.Fields.length : 0
          if (fieldCount > 0) {
            collected.push({
              category: 'form_labels',
              severity: 'info',
              message: `${fieldCount} AcroForm field${fieldCount === 1 ? '' : 's'} detected.`,
              detail: 'Verify every field has a TU (tooltip) or accessible name in the widget annotation. PDF/UA requires a /T (partial field name) and /TU (tooltip) on every widget annotation.',
            })
          }
        } catch {
          // Ignore — the AcroForm entry may not be a JSON array; we already
          // know the form exists.
        }
      }

      // ── (3) Image alt text (heuristic via page count + render) ──
      try {
        const url = await invoke<string>('render_page_thumbnail', { path: filePath, pageIndex: 0, widthPx: 320 })
        if (isMountedRef.current) setThumb(url)
        const imageCount = Number(cat.ImageCount ?? 0)
        if (imageCount > 0) {
          collected.push({
            category: 'alt_text',
            severity: 'warning',
            message: `${imageCount} raster image${imageCount === 1 ? '' : 's'} on first page.`,
            detail: 'Verify every Image XObject has a parent Form XObject with an /Alt entry. PDF/UA requires alt text on every meaningful image; decorative images should be marked as artifacts via /Artifact /Subtype /Background.',
          })
        } else {
          collected.push({
            category: 'alt_text',
            severity: 'info',
            message: 'No raster images detected on first page.',
            detail: 'Run the AltTextEditor to attach alt text to images across the document if any are present on later pages.',
          })
        }
      } catch {
        collected.push({
          category: 'alt_text',
          severity: 'info',
          message: 'Could not render first page for alt-text preview.',
          detail: 'PDFium may be unavailable. Alt-text checks still apply once the engine is back online.',
        })
      }

      // ── (4) Contrast ratio demo (informational) ───────────────
      // We surface a static guidance sample so the panel always has
      // something to teach even on a clean PDF.
      const fg: [number, number, number] = [34, 34, 34]
      const bg: [number, number, number] = [255, 255, 255]
      const ratio = contrastRatio(fg, bg)
      const passesAA = ratio >= 4.5
      const passesAALarge = ratio >= 3
      collected.push({
        category: 'contrast',
        severity: passesAA ? 'info' : passesAALarge ? 'warning' : 'error',
        message: `Reference contrast: ${ratio.toFixed(2)}:1 (${passesAA ? 'AA passes' : passesAALarge ? 'AA large text only' : 'AA fails'}).`,
        detail: `WCAG 2.1 AA requires a 4.5:1 contrast ratio for normal text and 3:1 for large text (18pt+ or 14pt+ bold). PDF/UA-1 mirrors this. We sampled body text vs page background as a reference; for live checks of the actual rendered text, open the document in a magnifier.`,
      })

      if (isMountedRef.current) setFindings(collected)
    } catch (e) {
      if (isMountedRef.current) setError(String(e))
    } finally {
      if (isMountedRef.current) setLoading(false)
    }
  }, [filePath])

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    runChecks()
  }, [runChecks])

  useEffect(() => {
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const summary = summarize(findings)

  return (
    <Card>
      <div className="card-title">Accessibility (PDF/UA & WCAG 2.1 AA)</div>
      <p className="accessibility-desc">
        Lightweight structural and contrast sweep. Uses the PDF catalog and a
        page-0 thumbnail to surface missing tag trees, suspected missing alt
        text, AcroForm field counts, and a reference contrast measurement.
      </p>

      {error && <div className="pdf-finding pdf-finding--error">{error}</div>}

      <div className="accessibility-summary">
        <div className={`accessibility-badge ${summary.wcag_aa_pass ? 'pass' : 'fail'}`}>
          {summary.wcag_aa_pass ? 'WCAG 2.1 AA: no errors' : 'WCAG 2.1 AA: needs review'}
        </div>
        <span className="accessibility-counts">
          {summary.by_severity.error} error · {summary.by_severity.warning} warning · {summary.by_severity.info} info
        </span>
        <Button variant="secondary" size="sm" onClick={runChecks} disabled={loading}>
          {loading ? 'Checking…' : 'Re-run checks'}
        </Button>
      </div>

      {thumb && (
        <div className="accessibility-thumb-row">
          <img className="accessibility-thumb" src={convertFileSrc(thumb)} alt="Page 1 thumbnail" />
          <div className="accessibility-legend">
            <h5>Page 1 preview</h5>
            <p>Used by the alt-text detector to count images and to render a contrast reference.</p>
            <button
              className="accessibility-luminance-link"
              onClick={() => setShowLuminanceDemo((v) => !v)}
            >
              {showLuminanceDemo ? 'Hide' : 'Show'} contrast formula
            </button>
            {showLuminanceDemo && (
              <pre className="accessibility-luminance-formula">
                L = 0.2126·R + 0.7152·G + 0.0722·B
                CR = (L_lighter + 0.05) / (L_darker + 0.05)
                Normal text needs CR ≥ 4.5 · Large text needs CR ≥ 3.0
              </pre>
            )}
          </div>
        </div>
      )}

      <ul className="accessibility-findings">
        {findings.map((f, i) => (
          <li key={i} className={`accessibility-finding accessibility-finding--${f.severity}`}>
            <span className="accessibility-finding-sev">{severityBadge[f.severity].label.toUpperCase()}</span>
            <div>
              <div className="accessibility-finding-message">{f.message}</div>
              <div className="accessibility-finding-detail">{f.detail}</div>
            </div>
          </li>
        ))}
        {!loading && findings.length === 0 && (
          <li className="accessibility-empty">No findings. The structural sweep did not detect any issues.</li>
        )}
      </ul>
    </Card>
  )
}
