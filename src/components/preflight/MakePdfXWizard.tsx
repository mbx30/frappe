import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { CombinedPreflightResult, SpotColorFinding } from '../../types'

type WizardStep = 'preflight' | 'review' | 'apply' | 'done'

interface MakePdfXWizardProps {
  filePath: string
  preflightResult: CombinedPreflightResult | null
  onRerunPreflight: () => Promise<void>
}

const PROFILE_OPTIONS = [
  { id: 'x4', label: 'PDF/X-4 (Recommended)', desc: 'Permits live transparency and ICC-based color. Best for modern workflows.' },
  { id: 'x1a', label: 'PDF/X-1a (Legacy)', desc: 'Flattens transparency and converts to CMYK. Required by some older RIPs.' },
  { id: 'general', label: 'General Print', desc: 'RGB and TAC>300% as warnings. No OutputIntent required. Best for small shops.' },
]

const ICC_PROFILES = [
  { value: 'FOGRA39-ISO-Coated-v2', label: 'FOGRA39 (ISO Coated v2)' },
  { value: 'FOGRA47-ISO-Uncoated-v3', label: 'FOGRA47 (ISO Uncoated v3)' },
  { value: 'FOGRA45-ISO-LWC-Improved', label: 'FOGRA45 (ISO LWC Improved)' },
]

export default function MakePdfXWizard({ filePath, preflightResult, onRerunPreflight }: MakePdfXWizardProps) {
  const [step, setStep] = useState<WizardStep>('preflight')
  const [profile, setProfile] = useState('x4')
  const [fixBleed, setFixBleed] = useState(true)
  const [fixOutputIntent, setFixOutputIntent] = useState(true)
  const [fixColors, setFixColors] = useState(true)
  const [iccProfile, setIccProfile] = useState(ICC_PROFILES[0].value)
  const [applying, setApplying] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [outputPath, setOutputPath] = useState<string | null>(null)
  const [spotColors, setSpotColors] = useState<SpotColorFinding[]>([])

  const hasBleedIssues = preflightResult?.bleed?.some(b => b.severity === 'error' || b.severity === 'warning') ?? false
  const hasNoOutputIntent = (preflightResult?.output_intents?.length ?? 0) === 0
  const hasRgbContent = preflightResult?.color_spaces?.some(
    cs => cs.color_space === 'DeviceRGB' || cs.color_space.startsWith('ICCBased')
  ) ?? false
  const totalErrors = [
    ...(preflightResult?.fonts ?? []),
    ...(preflightResult?.page_boxes ?? []),
    ...(preflightResult?.images ?? []),
    ...(preflightResult?.bleed ?? []),
    ...(preflightResult?.security ?? []),
    ...(preflightResult?.pdfx ?? []),
    ...(preflightResult?.color_spaces ?? []),
    ...(preflightResult?.overprint ?? []),
    ...(preflightResult?.transparency ?? []),
    ...(preflightResult?.hidden_content ?? []),
  ].filter(f => f.severity === 'error').length

  const handleRunPreflight = async () => {
    setStep('preflight')
    await onRerunPreflight()
  }

  const handleFetchSpotColors = async () => {
    try {
      const spots = await invoke<SpotColorFinding[]>('check_spot_colors', { path: filePath })
      setSpotColors(spots)
    } catch (e) {
      console.error('Spot color check failed:', e)
    }
  }

  const handleApply = async () => {
    setApplying(true)
    setError(null)
    try {
      const base = filePath.replace(/\.pdf$/i, '')
      let currentPath = filePath
      const steps: string[] = []

      if (fixBleed && hasBleedIssues) {
        const bleedOut = `${base}_bleed.pdf`
        await invoke('add_bleed', { path: currentPath, amountMm: 3, outputPath: bleedOut })
        currentPath = bleedOut
        steps.push('Added bleed (3mm)')
      }

      if (fixColors && hasRgbContent) {
        const cmykOut = `${base}_cmyk.pdf`
        await invoke('convert_rgb_to_cmyk', {
          path: currentPath,
          outputPath: cmykOut,
          scope: 'both',
          srcProfile: null,
          dstProfile: null,
          renderingIntent: null,
        })
        currentPath = cmykOut
        steps.push('Converted RGB→CMYK')
      }

      if (fixOutputIntent && hasNoOutputIntent && (profile === 'x4' || profile === 'x1a')) {
        if (!iccProfile) throw new Error('ICC profile is required for OutputIntent')
        const intentOut = `${base}_pdfx.pdf`
        const selectedProfileData = ICC_PROFILES.find(p => p.value === iccProfile)
        const conditionId = selectedProfileData?.label || 'FOGRA39 (ISO Coated v2)'
        const condition = selectedProfileData?.label || 'ISO Coated v2 (FOGRA39)'
        await invoke('add_output_intent', {
          path: currentPath,
          outputPath: intentOut,
          iccProfile,
          conditionId,
          condition,
        })
        currentPath = intentOut
        steps.push(`Added ${profile === 'x4' ? 'PDF/X-4' : 'PDF/X-1a'} OutputIntent`)
      }

      setOutputPath(currentPath)
      setStep('done')
    } catch (e) {
      setError(String(e))
    } finally {
      setApplying(false)
    }
  }

  if (!preflightResult) {
    return (
      <div className="pdfx-wizard">
        <div className="pdf-preflight-header">
          <h4>Make PDF/X Wizard</h4>
        </div>
        <p className="pdf-empty">Run a preflight check first to see what needs fixing.</p>
        <button className="btn btn-primary" onClick={handleRunPreflight}>Run Preflight</button>
      </div>
    )
  }

  return (
    <div className="pdfx-wizard">
      {step === 'preflight' && (
        <>
          <div className="pdfx-wizard-header">
            <h4>Step 1: Choose Target Profile</h4>
          </div>
          <div className="pdfx-profiles">
            {PROFILE_OPTIONS.map(p => (
              <label key={p.id} className={`pdfx-profile-option ${profile === p.id ? 'pdfx-profile-option--selected' : ''}`}>
                <input type="radio" name="profile" value={p.id} checked={profile === p.id} onChange={() => setProfile(p.id)} />
                <div className="pdfx-profile-content">
                  <span className="pdfx-profile-label">{p.label}</span>
                  <span className="pdfx-profile-desc">{p.desc}</span>
                </div>
              </label>
            ))}
          </div>

          <div className="pdfx-summary">
            <p>Preflight found <strong>{totalErrors} error(s)</strong>.</p>
            <ul>
              {hasBleedIssues && <li>⚠ Bleed issues detected — will auto-fix</li>}
              {hasNoOutputIntent && <li>⚠ No OutputIntent — will add for PDF/X</li>}
              {hasRgbContent && <li>⚠ RGB content — will convert to CMYK</li>}
              {!hasBleedIssues && !hasNoOutputIntent && !hasRgbContent && <li>✅ No auto-fixable issues found</li>}
            </ul>
          </div>

          <div className="pdfx-fixups">
            <h5>Auto-fix options:</h5>
            <label className="pdfx-fixup-row">
              <input type="checkbox" checked={fixBleed} onChange={e => setFixBleed(e.target.checked)} disabled={!hasBleedIssues} />
              Add bleed (3mm)
            </label>
            <label className="pdfx-fixup-row">
              <input type="checkbox" checked={fixOutputIntent} onChange={e => setFixOutputIntent(e.target.checked)} disabled={!hasNoOutputIntent} />
              {profile === 'x4' ? 'Embed PDF/X-4 OutputIntent' : 'Embed PDF/X-1a OutputIntent'}
            </label>
            {fixOutputIntent && hasNoOutputIntent && (
              <div className="pdfx-icc-selection">
                <label htmlFor="icc-profile">Select ICC Profile:</label>
                <select
                  id="icc-profile"
                  value={iccProfile}
                  onChange={e => setIccProfile(e.target.value)}
                  className="pdfx-select"
                >
                  {ICC_PROFILES.map(p => (
                    <option key={p.value} value={p.value}>{p.label}</option>
                  ))}
                </select>
              </div>
            )}
            <label className="pdfx-fixup-row">
              <input type="checkbox" checked={fixColors} onChange={e => setFixColors(e.target.checked)} disabled={!hasRgbContent} />
              Convert RGB→CMYK
            </label>
          </div>

          <div className="pdfx-wizard-actions">
            <button className="btn btn-secondary" onClick={handleFetchSpotColors}>Spot Color Inventory</button>
            <button className="btn btn-primary" onClick={() => setStep('review')}>Review & Apply</button>
          </div>

          {spotColors.length > 0 && (
            <div className="spot-inventory">
              <h5>Spot Colors</h5>
              {spotColors.map((s, i) => (
                <div key={i} className={`pdf-finding pdf-finding--${s.severity}`}>
                  <span className="pdf-finding-name">{s.name}</span>
                  <span className="pdf-finding-type">{s.alternate_colorspace_type}</span>
                  <span className="pdf-finding-pages">p. {s.pages.join(', ')}</span>
                  <span className="pdf-finding-message">{s.message}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {step === 'review' && (
        <>
          <div className="pdfx-wizard-header">
            <h4>Step 2: Review Changes</h4>
          </div>
          <div className="pdfx-review">
            <p><strong>Target:</strong> {PROFILE_OPTIONS.find(p => p.id === profile)?.label}</p>
            <ul>
              {fixBleed && hasBleedIssues && <li>Add 3mm bleed</li>}
              {fixOutputIntent && hasNoOutputIntent && <li>Add OutputIntent ({profile === 'x4' ? 'PDF/X-4' : 'PDF/X-1a'}) with {ICC_PROFILES.find(p => p.value === iccProfile)?.label}</li>}
              {fixColors && hasRgbContent && <li>Convert RGB objects to CMYK</li>}
              {!fixBleed && !fixOutputIntent && !fixColors && <li>No fixups selected — running preflight only</li>}
            </ul>
            <p className="pdfx-review-note">A new suffixed file will be created. The original will not be modified.</p>
          </div>
          <div className="pdfx-wizard-actions">
            <button className="btn btn-secondary" onClick={() => setStep('preflight')}>Back</button>
            <button className="btn btn-primary" onClick={handleApply} disabled={applying || (fixOutputIntent && hasNoOutputIntent && !iccProfile)}>
              {applying ? 'Applying...' : 'Generate PDF/X'}
            </button>
          </div>
          {error && <div className="pdf-finding pdf-finding--error">{error}</div>}
        </>
      )}

      {step === 'done' && (
        <div className="pdfx-done">
          <h4>✓ PDF/X Generated</h4>
          <p>The PDF has been processed and saved as:</p>
          <code className="pdfx-output-path">{outputPath}</code>
          <div className="pdfx-wizard-actions">
            <button className="btn btn-primary" onClick={() => { setStep('preflight'); setOutputPath(null); onRerunPreflight() }}>
              Run Preflight on Output
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
