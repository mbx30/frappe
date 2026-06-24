import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { IccProfileInfo, CombinedPreflightResult, ConversionResult } from '../../types'
import { t } from '../../i18n'

const RENDERING_INTENTS = [
  { id: 'perceptual', labelKey: 'color.intent.perceptual', descKey: 'color.intent.perceptual.desc' },
  { id: 'relative_colorimetric', labelKey: 'color.intent.relative', descKey: 'color.intent.relative.desc' },
  { id: 'absolute_colorimetric', labelKey: 'color.intent.absolute', descKey: 'color.intent.absolute.desc' },
  { id: 'saturation', labelKey: 'color.intent.saturation', descKey: 'color.intent.saturation.desc' },
]

const SCOPES = [
  { id: 'both', labelKey: 'color.scope.both' },
  { id: 'images', labelKey: 'color.scope.images' },
  { id: 'vector', labelKey: 'color.scope.vector' },
]

interface ColorConversionPanelProps {
  filePath: string
  preflightResult: CombinedPreflightResult | null
}

export default function ColorConversionPanel({ filePath, preflightResult }: ColorConversionPanelProps) {
  const [profiles, setProfiles] = useState<IccProfileInfo[]>([])
  const [srcProfile, setSrcProfile] = useState('sRGB_v4')
  const [dstProfile, setDstProfile] = useState('ISOcoated_v2')
  const [intent, setIntent] = useState('perceptual')
  const [scope, setScope] = useState('both')
  const [converting, setConverting] = useState(false)
  const [result, setResult] = useState<ConversionResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    invoke<IccProfileInfo[]>('list_icc_profiles').then(setProfiles).catch(() => {})
  }, [])

  const handleConvert = async () => {
    setConverting(true)
    setError(null)
    setResult(null)
    try {
      const outputPath = filePath.replace(/\.pdf$/i, '_CMYK.pdf')
      const res = await invoke<ConversionResult>('convert_rgb_to_cmyk', {
        path: filePath,
        outputPath,
        scope,
        srcProfile,
        dstProfile,
        renderingIntent: intent,
      })
      setResult(res)
    } catch (e) {
      setError(String(e))
    } finally {
      setConverting(false)
    }
  }

  const needsConversion = preflightResult?.color_spaces?.some(
    (cs) => cs.color_space === 'DeviceRGB' || cs.color_space.startsWith('ICCBased')
  ) ?? false

  return (
    <div className="color-conversion-panel">
      <div className="pdf-preflight-header">
        <h4>{t('color.title')}</h4>
        {needsConversion && <span className="pdf-badge pdf-badge-error">{t('color.rgb_detected')}</span>}
      </div>

      {!needsConversion && (
        <p className="pdf-empty">{t('color.not_needed')}</p>
      )}

      <div className="conversion-form">
        <div className="conversion-field">
          <label className="pdf-label">{t('color.src_profile')}</label>
          <select value={srcProfile} onChange={e => setSrcProfile(e.target.value)} className="form-select">
            {profiles.filter(p => p.color_space_type === 'RGB' || p.color_space_type === 'GRAY').map(p => (
              <option key={p.name} value={p.name}>{p.description}</option>
            ))}
            <option value="sRGB_v4">sRGB v4 (built-in)</option>
          </select>
        </div>

        <div className="conversion-field">
          <label className="pdf-label">{t('color.dst_profile')}</label>
          <select value={dstProfile} onChange={e => setDstProfile(e.target.value)} className="form-select">
            {profiles.filter(p => p.color_space_type === 'CMYK').map(p => (
              <option key={p.name} value={p.name}>{p.description}</option>
            ))}
          </select>
        </div>

        <div className="conversion-field">
          <label className="pdf-label">{t('color.intent')}</label>
          <select value={intent} onChange={e => setIntent(e.target.value)} className="form-select">
            {RENDERING_INTENTS.map(ri => (
              <option key={ri.id} value={ri.id}>{t(ri.labelKey)}</option>
            ))}
          </select>
          <span className="conversion-hint">{RENDERING_INTENTS.find(ri => ri.id === intent) ? t(RENDERING_INTENTS.find(ri => ri.id === intent)!.descKey) : ''}</span>
        </div>

        <div className="conversion-field">
          <label className="pdf-label">{t('color.scope')}</label>
          <div className="conversion-scopes">
            {SCOPES.map(s => (
              <label key={s.id} className="conversion-scope-radio">
                <input type="radio" name="scope" value={s.id} checked={scope === s.id} onChange={() => setScope(s.id)} />
                {t(s.labelKey)}
              </label>
            ))}
          </div>
        </div>

        <button className="btn btn-primary" onClick={handleConvert} disabled={converting || !needsConversion}>
          {converting ? t('color.converting') : t('color.convert')}
        </button>

        {error && <div className="pdf-finding pdf-finding--error">{error}</div>}

        {result && (
          <div className="conversion-result">
            <p className="conversion-result-header">{t('color.complete')}</p>
            <p>{t('color.images_converted', { n: result.images_converted })}</p>
            <p>{t('color.vector_converted', { n: result.vector_ops_converted })}</p>
            {result.warnings.length > 0 && (
              <div className="conversion-warnings">
                {result.warnings.map((w, i) => (
                  <p key={i} className="pdf-finding pdf-finding--warning">{w}</p>
                ))}
              </div>
            )}
            <p className="conversion-output">{t('color.saved_as', { path: filePath.replace(/\.pdf$/i, '_CMYK.pdf') })}</p>
          </div>
        )}
      </div>
    </div>
  )
}
