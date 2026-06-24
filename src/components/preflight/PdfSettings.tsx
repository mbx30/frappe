import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Input, Select, Card } from '../../design-system'
import { t } from '../../i18n'

const PDF_X_OPTIONS = [
  { value: 'PDF/X-1a:2001', label: 'PDF/X-1a:2001' },
  { value: 'PDF/X-1a:2003', label: 'PDF/X-1a:2003' },
  { value: 'PDF/X-3:2003', label: 'PDF/X-3:2003' },
  { value: 'PDF/X-4:2010', label: 'PDF/X-4:2010' },
  { value: 'PDF/X-4p', label: 'PDF/X-4p (process)' },
  { value: 'PDF/X-5g', label: 'PDF/X-5g (external graphics)' },
]

const ICC_PROFILES = [
  { value: 'FOGRA39-ISO-Coated-v2', label: 'FOGRA39 (ISO Coated v2)' },
  { value: 'FOGRA47-ISO-Uncoated-v3', label: 'FOGRA47 (ISO Uncoated v3)' },
  { value: 'FOGRA45-ISO-LWC-Improved', label: 'FOGRA45 (ISO LWC Improved)' },
  { value: 'GRACoL-Coated-v3', label: 'GRACoL Coated v3' },
  { value: 'SWOP-Coated-v5', label: 'SWOP Coated v5' },
  { value: 'sRGB', label: 'sRGB IEC61966-2.1' },
]

interface PdfSettings {
  default_bleed_mm: number
  default_dpi_threshold: number
  default_icc_profile: string
  default_pdfx_standard: string
}

const DEFAULTS: PdfSettings = {
  default_bleed_mm: 3,
  default_dpi_threshold: 300,
  default_icc_profile: 'FOGRA39-ISO-Coated-v2',
  default_pdfx_standard: 'PDF/X-4:2010',
}

export default function PdfSettings() {
  const [settings, setSettings] = useState<PdfSettings>(DEFAULTS)
  const [original, setOriginal] = useState<PdfSettings>(DEFAULTS)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const all = await invoke<Record<string, string>>('get_all_preferences')
        if (cancelled) return
        const next: PdfSettings = { ...DEFAULTS }
        for (const key of Object.keys(DEFAULTS) as (keyof PdfSettings)[]) {
          const value = all[key]
          if (value !== undefined && value !== '') {
            if (key === 'default_bleed_mm' || key === 'default_dpi_threshold') {
              ;(next[key] as number) = Number(value) || DEFAULTS[key]
            } else {
              ;(next[key] as string) = value
            }
          }
        }
        setSettings(next)
        setOriginal(next)
      } catch (e) {
        setMessage(`Failed to load settings: ${e}`)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => { cancelled = true }
  }, [])

  const isDirty =
    settings.default_bleed_mm !== original.default_bleed_mm ||
    settings.default_dpi_threshold !== original.default_dpi_threshold ||
    settings.default_icc_profile !== original.default_icc_profile ||
    settings.default_pdfx_standard !== original.default_pdfx_standard

  const handleSave = async () => {
    setSaving(true)
    setMessage(null)
    try {
      for (const key of Object.keys(settings) as (keyof PdfSettings)[]) {
        await invoke('set_preference', { key, value: String(settings[key]) })
      }
      setOriginal(settings)
      setMessage('Settings saved.')
    } catch (e) {
      setMessage(`Save failed: ${e}`)
    } finally {
      setSaving(false)
    }
  }

  const handleReset = () => {
    setSettings(original)
    setMessage(null)
  }

  if (loading) {
    return <Card><div className="pdf-empty">Loading settings…</div></Card>
  }

  return (
    <Card>
      <div className="card-title">{t('pdfsettings.title')}</div>
      <p className="pdf-settings-desc">
        {t('pdfsettings.desc')}
      </p>

      <div className="pdf-settings-grid">
        <div className="conversion-field">
          <Input
            type="number"
            label={t('pdfsettings.bleed.label')}
            value={settings.default_bleed_mm}
            min={0}
            max={20}
            step={0.5}
            onChange={(e) => setSettings({ ...settings, default_bleed_mm: Number(e.target.value) })}
            hint={t('pdfsettings.bleed.hint')}
          />
        </div>

        <div className="conversion-field">
          <Input
            type="number"
            label={t('pdfsettings.dpi.label')}
            value={settings.default_dpi_threshold}
            min={72}
            max={1200}
            step={10}
            onChange={(e) => setSettings({ ...settings, default_dpi_threshold: Number(e.target.value) })}
            hint={t('pdfsettings.dpi.hint')}
          />
        </div>

        <div className="conversion-field">
          <label className="pdf-label">{t('pdfsettings.icc.label')}</label>
          <Select
            value={settings.default_icc_profile}
            onChange={(e) => setSettings({ ...settings, default_icc_profile: e.target.value })}
            options={ICC_PROFILES.map((p) => ({ value: p.value, label: p.label }))}
          />
        </div>

        <div className="conversion-field">
          <label className="pdf-label">{t('pdfsettings.pdfx.label')}</label>
          <Select
            value={settings.default_pdfx_standard}
            onChange={(e) => setSettings({ ...settings, default_pdfx_standard: e.target.value })}
            options={PDF_X_OPTIONS}
          />
        </div>
      </div>

      <div className="pdf-settings-actions">
        <button
          className="btn btn-secondary"
          onClick={handleReset}
          disabled={!isDirty || saving}
        >
          {t('common.reset') ?? 'Reset'}
        </button>
        <button
          className="btn btn-primary"
          onClick={handleSave}
          disabled={!isDirty || saving}
        >
          {saving ? t('common.saving') ?? 'Saving…' : t('common.save') ?? 'Save'}
        </button>
      </div>

      {message && <p className="pdf-settings-message" role="status">{message}</p>}
    </Card>
  )
}
