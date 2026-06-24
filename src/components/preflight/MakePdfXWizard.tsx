import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { CombinedPreflightResult, SpotColorFinding } from '../../types'
import { t } from '../../i18n'

type WizardStep = 'preflight' | 'review' | 'apply' | 'done'

interface MakePdfXWizardProps {
  filePath: string
  preflightResult: CombinedPreflightResult | null
  onRerunPreflight: () => Promise<void>
}

const PROFILE_OPTIONS = [
  { id: 'x4', labelKey: 'wizard.profile.x4', descKey: 'wizard.profile.x4.desc' },
  { id: 'x1a', labelKey: 'wizard.profile.x1a', descKey: 'wizard.profile.x1a.desc' },
  { id: 'general', labelKey: 'wizard.profile.general', descKey: 'wizard.profile.general.desc' },
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
          <h4>{t('wizard.title')}</h4>
        </div>
        <p className="pdf-empty">{t('wizard.run_preflight_first')}</p>
        <button className="btn btn-primary" onClick={handleRunPreflight}>{t('wizard.run_preflight')}</button>
      </div>
    )
  }

  return (
    <div className="pdfx-wizard">
      {step === 'preflight' && (
        <>
          <div className="pdfx-wizard-header">
            <h4>{t('wizard.step.profile')}</h4>
          </div>
          <div className="pdfx-profiles">
            {PROFILE_OPTIONS.map(p => (
              <label key={p.id} className={`pdfx-profile-option ${profile === p.id ? 'pdfx-profile-option--selected' : ''}`}>
                <input type="radio" name="profile" value={p.id} checked={profile === p.id} onChange={() => setProfile(p.id)} />
                <div className="pdfx-profile-content">
                  <span className="pdfx-profile-label">{t(p.labelKey)}</span>
                  <span className="pdfx-profile-desc">{t(p.descKey)}</span>
                </div>
              </label>
            ))}
          </div>

          <div className="pdfx-summary">
            <p>{t('wizard.errors_found', { n: totalErrors })}</p>
            <ul>
              {hasBleedIssues && <li>{t('wizard.bleed_warning')}</li>}
              {hasNoOutputIntent && <li>{t('wizard.no_intent')}</li>}
              {hasRgbContent && <li>{t('wizard.rgb_warning')}</li>}
              {!hasBleedIssues && !hasNoOutputIntent && !hasRgbContent && <li>{t('wizard.no_issues')}</li>}
            </ul>
          </div>

          <div className="pdfx-fixups">
            <h5>{t('wizard.fixups')}</h5>
            <label className="pdfx-fixup-row">
              <input type="checkbox" checked={fixBleed} onChange={e => setFixBleed(e.target.checked)} disabled={!hasBleedIssues} />
              {t('wizard.fix.bleed')}
            </label>
            <label className="pdfx-fixup-row">
              <input type="checkbox" checked={fixOutputIntent} onChange={e => setFixOutputIntent(e.target.checked)} disabled={!hasNoOutputIntent} />
              {t('wizard.fix.intent', { profile: profile === 'x4' ? 'PDF/X-4' : 'PDF/X-1a' })}
            </label>
            {fixOutputIntent && hasNoOutputIntent && (
              <div className="pdfx-icc-selection">
                <label htmlFor="icc-profile">{t('wizard.icc')}</label>
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
              {t('wizard.fix.rgb')}
            </label>
          </div>

          <div className="pdfx-wizard-actions">
            <button className="btn btn-secondary" onClick={handleFetchSpotColors}>{t('wizard.spot_inventory')}</button>
            <button className="btn btn-primary" onClick={() => setStep('review')}>{t('wizard.review_apply')}</button>
          </div>

          {spotColors.length > 0 && (
            <div className="spot-inventory">
              <h5>{t('wizard.spots')}</h5>
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
            <h4>{t('wizard.step.review')}</h4>
          </div>
          <div className="pdfx-review">
            <p><strong>{t('wizard.target')}:</strong> {PROFILE_OPTIONS.find(p => p.id === profile) ? t(PROFILE_OPTIONS.find(p => p.id === profile)!.labelKey) : ''}</p>
            <ul>
              {fixBleed && hasBleedIssues && <li>{t('wizard.review.bleed')}</li>}
              {fixOutputIntent && hasNoOutputIntent && <li>{t('wizard.review.intent', { profile: profile === 'x4' ? 'PDF/X-4' : 'PDF/X-1a', icc: ICC_PROFILES.find(p => p.value === iccProfile)?.label ?? '' })}</li>}
              {fixColors && hasRgbContent && <li>{t('wizard.review.rgb')}</li>}
              {!fixBleed && !fixOutputIntent && !fixColors && <li>{t('wizard.review.none')}</li>}
            </ul>
            <p className="pdfx-review-note">{t('wizard.review.note')}</p>
          </div>
          <div className="pdfx-wizard-actions">
            <button className="btn btn-secondary" onClick={() => setStep('preflight')}>{t('wizard.back')}</button>
            <button className="btn btn-primary" onClick={handleApply} disabled={applying || (fixOutputIntent && hasNoOutputIntent && !iccProfile)}>
              {applying ? t('wizard.applying') : t('wizard.generate')}
            </button>
          </div>
          {error && <div className="pdf-finding pdf-finding--error">{error}</div>}
        </>
      )}

      {step === 'done' && (
        <div className="pdfx-done">
          <h4>{t('wizard.done')}</h4>
          <p>{t('wizard.done.desc')}</p>
          <code className="pdfx-output-path">{outputPath}</code>
          <div className="pdfx-wizard-actions">
            <button className="btn btn-primary" onClick={() => { setStep('preflight'); setOutputPath(null); onRerunPreflight() }}>
              {t('wizard.rerun_output')}
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
