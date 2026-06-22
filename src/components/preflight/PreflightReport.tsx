import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { CombinedPreflightResult, BleedFinding } from '../../types'
import BleedCheck from './BleedCheck'
import { t } from '../../i18n'

interface PreflightReportProps {
  filePath: string
  result: CombinedPreflightResult
  jobId: number | null
  onSaved: () => void
}

function countBySeverity(items: { severity: string }[]): { errors: number; warnings: number; ok: number } {
  let errors = 0, warnings = 0, ok = 0
  for (const item of items) {
    if (item.severity === 'error') errors++
    else if (item.severity === 'warning') warnings++
    else ok++
  }
  return { errors, warnings, ok }
}

type SectionState = Record<string, boolean>

const PROFILES = [
  { id: 'full', label: 'Full Check' },
  { id: 'x4', label: 'PDF/X-4' },
  { id: 'x1a', label: 'PDF/X-1a' },
  { id: 'x3', label: 'PDF/X-3' },
]

export default function PreflightReport({ filePath, result, jobId, onSaved }: PreflightReportProps) {
  const [sections, setSections] = useState<SectionState>({
    fonts: true, boxes: true, images: true, bleed: true, intents: true, security: true, pdfx: true, color_spaces: true, spot_colors: true, overprint: true, transparency: true, hidden_content: true,
  })
  const [minBleed, setMinBleed] = useState(3)
  const [bleedFindings, setBleedFindings] = useState(result.bleed)
  const [saving, setSaving] = useState(false)
  const [saveMsg, setSaveMsg] = useState<string | null>(null)
  const [profile, setProfile] = useState('full')
  const [running, setRunning] = useState(false)

  const toggle = (key: string) => setSections(prev => ({ ...prev, [key]: !prev[key] }))

  const fc = countBySeverity(result.fonts)
  const bc = countBySeverity(result.page_boxes)
  const ic = countBySeverity(result.images)
  const blc = countBySeverity(bleedFindings)
  const sc = countBySeverity(result.security)
  const xc = countBySeverity(result.pdfx)
  const cc = countBySeverity(result.color_spaces)
  const oc = countBySeverity(result.overprint)
  const tc = countBySeverity(result.transparency)
  const hc = countBySeverity(result.hidden_content)

  const totalErrors = fc.errors + bc.errors + ic.errors + blc.errors + sc.errors + xc.errors + cc.errors + oc.errors + tc.errors + hc.errors
  const totalWarnings = fc.warnings + bc.warnings + ic.warnings + blc.warnings + sc.warnings + xc.warnings + cc.warnings + oc.warnings + tc.warnings + hc.warnings

  const handleSave = async () => {
    if (!jobId) return
    setSaving(true)
    setSaveMsg(null)
    try {
      const findings: Array<Record<string, unknown>> = []
      for (const f of result.fonts) findings.push({ check_name: 'fonts', severity: f.severity, page_num: null, object_ref: null, message: f.message, fix_hint: '' })
      for (const f of result.page_boxes) findings.push({ check_name: 'page_boxes', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: null, message: f.message, fix_hint: '' })
      for (const f of result.images) findings.push({ check_name: 'image_resolution', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.image_name, message: f.message, fix_hint: '' })
      for (const f of bleedFindings) findings.push({ check_name: 'bleed', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: null, message: f.message, fix_hint: '' })
      for (const f of result.security) findings.push({ check_name: 'security', severity: f.severity, page_num: null, object_ref: f.category, message: f.message, fix_hint: '' })
      for (const f of result.pdfx) findings.push({ check_name: 'pdfx', severity: f.severity, page_num: null, object_ref: f.category, message: f.message, fix_hint: f.fix_hint })
      for (const f of result.color_spaces) findings.push({ check_name: 'color_spaces', severity: f.severity, page_num: null, object_ref: f.color_space, message: f.message, fix_hint: '' })
      for (const f of result.overprint) findings.push({ check_name: 'overprint', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.object_context, message: f.message, fix_hint: '' })
      for (const f of result.transparency) findings.push({ check_name: 'transparency', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.ty, message: f.message, fix_hint: '' })
      for (const f of result.hidden_content) findings.push({ check_name: 'hidden_content', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.ty, message: f.description, fix_hint: '' })
      await invoke('save_preflight_run', { jobId, profile, findings })
      setSaveMsg('Report saved!')
      onSaved()
    } catch (e) {
      setSaveMsg(`Save failed: ${e}`)
    } finally {
      setSaving(false)
    }
  }

  const handleRunProfile = async () => {
    setRunning(true)
    try {
      await invoke('check_pdfx', { path: filePath, profile: profile === 'full' ? 'x4' : profile })
    } catch (e) {
      console.error('Profile check failed:', e)
    } finally {
      setRunning(false)
    }
  }

  return (
    <div className="pdf-preflight" role="region" aria-label="Preflight report">
      <div className="pdf-preflight-banner" role="status" aria-live="polite">
        {totalErrors > 0 || totalWarnings > 0 ? (
          <span className="pdf-preflight-status pdf-preflight-status--fail">
            {t('pdf.preflight_fail', { errors: totalErrors, s: totalErrors !== 1 ? 's' : '', warnings: totalWarnings, ws: totalWarnings !== 1 ? 's' : '' })}
          </span>
        ) : (
          <span className="pdf-preflight-status pdf-preflight-status--pass">{t('pdf.preflight_pass')}</span>
        )}
        <div className="pdf-preflight-controls">
          <select value={profile} onChange={e => setProfile(e.target.value)} className="form-select" aria-label="Preflight profile">
            {PROFILES.map(p => <option key={p.id} value={p.id}>{p.label}</option>)}
          </select>
          <button className="btn btn-secondary" onClick={handleRunProfile} disabled={running}>
            {running ? t('pdf.running_preflight') : t('pdf.run_check')}
          </button>
          <button className="btn btn-secondary" onClick={handleSave} disabled={saving || !jobId} aria-label={t('pdf.save_report')}>
            {saving ? t('pdf.saving') : t('pdf.save_report')}
          </button>
        </div>
        {saveMsg && <span className="pdf-preflight-save-msg" role="alert">{saveMsg}</span>}
      </div>

      {/* Font Checks */}
      <div className="pdf-preflight-section" role="region" aria-label="Font checks">
        <div className="pdf-preflight-header" onClick={() => toggle('fonts')} role="button" tabIndex={0} aria-expanded={sections.fonts} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggle('fonts') }}>
          <h4>Font Checks ({fc.errors}E, {fc.warnings}W, {fc.ok}OK)</h4>
          <span aria-hidden="true">{sections.fonts ? '▼' : '▶'}</span>
        </div>
        {sections.fonts && result.fonts.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">{f.font_name}</span>
            <span className="pdf-finding-type">({f.font_type})</span>
            <span className="pdf-finding-pages">p. {f.pages.join(', ')}</span>
            <span className="pdf-finding-message">{f.message}</span>
          </div>
        ))}
      </div>

      {/* Page Box Checks */}
      <div className="pdf-preflight-section" role="region" aria-label="Page box checks">
        <div className="pdf-preflight-header" onClick={() => toggle('boxes')} role="button" tabIndex={0} aria-expanded={sections.boxes} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggle('boxes') }}>
          <h4>Page Box Checks ({bc.errors}E, {bc.warnings}W, {bc.ok}OK)</h4>
          <span aria-hidden="true">{sections.boxes ? '▼' : '▶'}</span>
        </div>
        {sections.boxes && result.page_boxes.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">P.{f.page} {f.box_type}</span>
            <span className="pdf-finding-message">{f.message}</span>
          </div>
        ))}
      </div>

      {/* Image Resolution */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('images')} style={{ cursor: 'pointer' }}>
          <h4>Image Resolution ({ic.errors}E, {ic.warnings}W, {ic.ok}OK)</h4>
          <span>{sections.images ? '▼' : '▶'}</span>
        </div>
        {sections.images && result.images.map((f, i) => {
          const sev = f.effective_dpi < 150 ? 'error' : f.effective_dpi < 300 ? 'warning' : 'ok'
          return (
            <div key={i} className={`pdf-finding pdf-finding--${sev}`}>
              <span className="pdf-finding-sev">{sev.toUpperCase()}</span>
              <span className="pdf-finding-name">P.{f.page} {f.image_name}</span>
              <span className="pdf-finding-type">{f.pixel_width}×{f.pixel_height}px / {f.color_space}</span>
              <span className="pdf-finding-message">{f.effective_dpi.toFixed(0)} DPI — {f.message}</span>
            </div>
          )
        })}
      </div>

      {/* Bleed */}
      <BleedCheck
        filePath={filePath}
        findings={bleedFindings}
        minBleedMm={minBleed}
        onMinBleedChange={setMinBleed}
        onRerun={async (mm) => {
          try {
            const res = await invoke<BleedFinding[]>('check_bleed', { path: filePath, minBleedMm: mm })
            setBleedFindings(res)
          } catch { /* intentional */ }
        }}
      />

      {/* Output Intents */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('intents')} style={{ cursor: 'pointer' }}>
          <h4>Output Intents ({result.output_intents.length} found)</h4>
          <span>{sections.intents ? '▼' : '▶'}</span>
        </div>
        {sections.intents && result.output_intents.length === 0 && (
          <p className="pdf-empty">No output intents found (not PDF/X compliant)</p>
        )}
        {sections.intents && result.output_intents.map((o, i) => (
          <div key={i} className="pdf-finding pdf-finding--ok">
            <span className="pdf-finding-name">{o.s_key}</span>
            <span className="pdf-finding-type">{o.output_condition_id}</span>
            <span className="pdf-finding-message">
              ICC: {o.has_embedded_icc ? `${o.icc_num_channels}ch` : 'none'}
            </span>
          </div>
        ))}
      </div>

      {/* Security */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('security')} style={{ cursor: 'pointer' }}>
          <h4>Security Checks ({sc.errors}E, {sc.warnings}W, {sc.ok}OK)</h4>
          <span>{sections.security ? '▼' : '▶'}</span>
        </div>
        {sections.security && result.security.length === 0 && (
          <p className="pdf-empty">No security issues found.</p>
        )}
        {sections.security && result.security.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">{f.category}</span>
            <span className="pdf-finding-message">{f.message}</span>
          </div>
        ))}
      </div>

      {/* Color Spaces */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('color_spaces')} style={{ cursor: 'pointer' }}>
          <h4>Color Spaces ({cc.errors}E, {cc.warnings}W, {cc.ok}OK)</h4>
          <span>{sections.color_spaces ? '▼' : '▶'}</span>
        </div>
        {sections.color_spaces && result.color_spaces.length === 0 && (
          <p className="pdf-empty">No color space information available.</p>
        )}
        {sections.color_spaces && result.color_spaces.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">{f.color_space}</span>
            <span className="pdf-finding-type">({f.kind})</span>
            <span className="pdf-finding-pages">p. {f.pages.join(', ')}</span>
            {f.is_pdf_x_violation && <span className="pdf-finding-badge">PDF/X viol.</span>}
            <span className="pdf-finding-message">{f.message}</span>
          </div>
        ))}
      </div>

      {/* Overprint */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('overprint')} style={{ cursor: 'pointer' }}>
          <h4>Overprint ({oc.errors}E, {oc.warnings}W, {oc.ok}OK)</h4>
          <span>{sections.overprint ? '▼' : '▶'}</span>
        </div>
        {sections.overprint && result.overprint.length === 0 && (
          <p className="pdf-empty">No overprint settings found.</p>
        )}
        {sections.overprint && result.overprint.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">P.{f.page}</span>
            <span className="pdf-finding-type">{f.object_context}</span>
            <span className="pdf-finding-message">{f.message}</span>
          </div>
        ))}
      </div>

      {/* Transparency */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('transparency')} style={{ cursor: 'pointer' }}>
          <h4>Transparency ({tc.errors}E, {tc.warnings}W, {tc.ok}OK)</h4>
          <span>{sections.transparency ? '▼' : '▶'}</span>
        </div>
        {sections.transparency && result.transparency.length === 0 && (
          <p className="pdf-empty">No live transparency detected.</p>
        )}
        {sections.transparency && result.transparency.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">P.{f.page}</span>
            <span className="pdf-finding-type">{f.ty}</span>
            {f.is_pdfx1a_violation && <span className="pdf-finding-badge">X-1a viol.</span>}
            <span className="pdf-finding-message">{f.message}</span>
          </div>
        ))}
      </div>

      {/* Hidden Content */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('hidden_content')} style={{ cursor: 'pointer' }}>
          <h4>Hidden Content ({hc.errors}E, {hc.warnings}W, {hc.ok}OK)</h4>
          <span>{sections.hidden_content ? '▼' : '▶'}</span>
        </div>
        {sections.hidden_content && result.hidden_content.length === 0 && (
          <p className="pdf-empty">No hidden content detected.</p>
        )}
        {sections.hidden_content && result.hidden_content.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">{f.ty}</span>
            {f.page > 0 && <span className="pdf-finding-pages">P.{f.page}</span>}
            <span className="pdf-finding-message">{f.description}</span>
          </div>
        ))}
      </div>

      {/* PDF/X Compliance */}
      <div className="pdf-preflight-section">
        <div className="pdf-preflight-header" onClick={() => toggle('pdfx')} style={{ cursor: 'pointer' }}>
          <h4>PDF/X Compliance ({xc.errors}E, {xc.warnings}W, {xc.ok}OK)</h4>
          <span>{sections.pdfx ? '▼' : '▶'}</span>
        </div>
        {sections.pdfx && result.pdfx.length === 0 && (
          <p className="pdf-empty">No PDF/X metadata checks available.</p>
        )}
        {sections.pdfx && result.pdfx.map((f, i) => (
          <div key={i} className={`pdf-finding pdf-finding--${f.severity}`}>
            <span className="pdf-finding-sev">{f.severity.toUpperCase()}</span>
            <span className="pdf-finding-name">{f.category}</span>
            <span className="pdf-finding-message">{f.message}</span>
            {f.fix_hint && <span className="pdf-finding-type">💡 {f.fix_hint}</span>}
          </div>
        ))}
      </div>
    </div>
  )
}
