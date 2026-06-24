import { useState, useEffect, type ChangeEvent } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card, Input, Switch, Select } from '../../design-system'
import { t } from '../../i18n'

interface Preferences {
  pdf_settings: {
    default_bleed_mm: number
    default_dpi_threshold: number
    default_icc_profile: string
    default_pdfx_standard: string
  }
  ai_visual_check: {
    enabled: boolean
    endpoint: string
  }
  telemetry: {
    opt_in: boolean
  }
  keyboard: {
    mac_shortcuts: boolean
  }
  ui: {
    default_locale: string
    show_advanced: boolean
  }
}

const DEFAULTS: Preferences = {
  pdf_settings: {
    default_bleed_mm: 3,
    default_dpi_threshold: 300,
    default_icc_profile: 'FOGRA39-ISO-Coated-v2',
    default_pdfx_standard: 'PDF/X-4:2010',
  },
  ai_visual_check: {
    enabled: false,
    endpoint: 'https://api.openai.com/v1/chat/completions',
  },
  telemetry: {
    opt_in: false,
  },
  keyboard: {
    mac_shortcuts: true,
  },
  ui: {
    default_locale: 'en',
    show_advanced: false,
  },
}

const PREF_KEYS = {
  pdf_settings: 'preferences.pdf_settings',
  ai_visual_check: 'preferences.ai_visual_check',
  telemetry: 'preferences.telemetry',
  keyboard: 'preferences.keyboard',
  ui: 'preferences.ui',
}

async function loadPreferences(): Promise<Preferences> {
  const out: Preferences = JSON.parse(JSON.stringify(DEFAULTS))
  for (const [section, key] of Object.entries(PREF_KEYS)) {
    try {
      const value = await invoke<string | null>('get_preference', { key })
      if (value) {
        out[section as keyof Preferences] = JSON.parse(value)
      }
    } catch {
      // Ignore — fall back to defaults for this section.
    }
  }
  return out
}

async function saveSection(section: keyof Preferences, value: unknown): Promise<void> {
  await invoke('set_preference', {
    key: PREF_KEYS[section],
    value: JSON.stringify(value),
  })
}

export default function PreferencesPanel() {
  const [prefs, setPrefs] = useState<Preferences | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [hasChanges, setHasChanges] = useState(false)

  useEffect(() => {
    loadPreferences().then(setPrefs).catch(() => setPrefs(DEFAULTS))
  }, [])

  const update = <K extends keyof Preferences>(section: K, patch: Partial<Preferences[K]>) => {
    setPrefs((prev) => {
      if (!prev) return prev
      return { ...prev, [section]: { ...prev[section], ...patch } }
    })
    setHasChanges(true)
    setMessage(null)
  }

  const save = async () => {
    if (!prefs) return
    setSaving(true)
    setMessage(null)
    try {
      await saveSection('pdf_settings', prefs.pdf_settings)
      await saveSection('ai_visual_check', prefs.ai_visual_check)
      await saveSection('telemetry', prefs.telemetry)
      await saveSection('keyboard', prefs.keyboard)
      await saveSection('ui', prefs.ui)
      setHasChanges(false)
      setMessage(t('preferences.saved'))
    } catch (e) {
      setMessage(`Save failed: ${e}`)
    } finally {
      setSaving(false)
    }
  }

  if (!prefs) {
    return <Card><div className="pdf-empty">Loading preferences…</div></Card>
  }

  return (
    <div className="preferences-panel">
      <Card>
        <div className="card-title">{t('preferences.title')}</div>
        <p className="preferences-desc">{t('preferences.desc')}</p>
      </Card>

      <Card>
        <h4>{t('preferences.pdf_settings')}</h4>
        <div className="preferences-grid">
          <Input
            type="number"
            label={t('preferences.bleed_mm')}
            value={prefs.pdf_settings.default_bleed_mm}
            min={0}
            max={20}
            step={0.5}
            onChange={(e: ChangeEvent<HTMLInputElement>) => update('pdf_settings', { default_bleed_mm: Number(e.target.value) })}
          />
          <Input
            type="number"
            label={t('preferences.dpi')}
            value={prefs.pdf_settings.default_dpi_threshold}
            min={72}
            max={1200}
            step={10}
            onChange={(e: ChangeEvent<HTMLInputElement>) => update('pdf_settings', { default_dpi_threshold: Number(e.target.value) })}
          />
          <div className="preferences-field">
            <label className="pdf-label">{t('preferences.icc')}</label>
            <Select
              value={prefs.pdf_settings.default_icc_profile}
              onChange={(e: ChangeEvent<HTMLSelectElement>) => update('pdf_settings', { default_icc_profile: e.target.value })}
              options={[
                { value: 'FOGRA39-ISO-Coated-v2', label: 'FOGRA39 (ISO Coated v2)' },
                { value: 'FOGRA47-ISO-Uncoated-v3', label: 'FOGRA47 (ISO Uncoated v3)' },
                { value: 'GRACoL-Coated-v3', label: 'GRACoL Coated v3' },
                { value: 'SWOP-Coated-v5', label: 'SWOP Coated v5' },
                { value: 'sRGB', label: 'sRGB IEC61966-2.1' },
              ]}
            />
          </div>
          <div className="preferences-field">
            <label className="pdf-label">{t('preferences.pdfx_standard')}</label>
            <Select
              value={prefs.pdf_settings.default_pdfx_standard}
              onChange={(e: ChangeEvent<HTMLSelectElement>) => update('pdf_settings', { default_pdfx_standard: e.target.value })}
              options={[
                { value: 'PDF/X-1a:2001', label: 'PDF/X-1a:2001' },
                { value: 'PDF/X-1a:2003', label: 'PDF/X-1a:2003' },
                { value: 'PDF/X-3:2003', label: 'PDF/X-3:2003' },
                { value: 'PDF/X-4:2010', label: 'PDF/X-4:2010' },
              ]}
            />
          </div>
        </div>
      </Card>

      <Card>
        <h4>{t('preferences.ai.title')}</h4>
        <p className="preferences-desc">{t('preferences.ai.desc')}</p>
        <div className="preferences-field preferences-field--row">
          <Switch
            label={t('preferences.ai.enable')}
            checked={prefs.ai_visual_check.enabled}
            onChange={(e: ChangeEvent<HTMLInputElement>) => update('ai_visual_check', { enabled: e.target.checked })}
          />
        </div>
        <Input
          label={t('preferences.ai.endpoint')}
          value={prefs.ai_visual_check.endpoint}
          onChange={(e: ChangeEvent<HTMLInputElement>) => update('ai_visual_check', { endpoint: e.target.value })}
          disabled={!prefs.ai_visual_check.enabled}
        />
      </Card>

      <Card>
        <h4>{t('preferences.telemetry.title')}</h4>
        <p className="preferences-desc">{t('preferences.telemetry.desc')}</p>
        <div className="preferences-field preferences-field--row">
          <Switch
            label={t('preferences.telemetry.opt_in')}
            checked={prefs.telemetry.opt_in}
            onChange={(e: ChangeEvent<HTMLInputElement>) => update('telemetry', { opt_in: e.target.checked })}
          />
        </div>
      </Card>

      <Card>
        <h4>{t('preferences.keyboard.title')}</h4>
        <div className="preferences-field preferences-field--row">
          <Switch
            label={t('preferences.keyboard.mac')}
            checked={prefs.keyboard.mac_shortcuts}
            onChange={(e: ChangeEvent<HTMLInputElement>) => update('keyboard', { mac_shortcuts: e.target.checked })}
          />
        </div>
      </Card>

      <Card>
        <h4>{t('preferences.ui.title')}</h4>
        <div className="preferences-field">
          <label className="pdf-label">{t('preferences.ui.locale')}</label>
          <Select
            value={prefs.ui.default_locale}
            onChange={(e: ChangeEvent<HTMLSelectElement>) => update('ui', { default_locale: e.target.value })}
            options={[
              { value: 'en', label: 'English' },
            ]}
          />
        </div>
        <div className="preferences-field preferences-field--row">
          <Switch
            label={t('preferences.ui.show_advanced')}
            checked={prefs.ui.show_advanced}
            onChange={(e: ChangeEvent<HTMLInputElement>) => update('ui', { show_advanced: e.target.checked })}
          />
        </div>
      </Card>

      <div className="preferences-actions">
        {message && <p className="preferences-message" role="status">{message}</p>}
        <Button variant="primary" onClick={save} disabled={!hasChanges || saving}>
          {saving ? t('common.saving') ?? 'Saving…' : t('common.save') ?? 'Save preferences'}
        </Button>
      </div>
    </div>
  )
}
