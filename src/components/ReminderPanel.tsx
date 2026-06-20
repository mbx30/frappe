import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Select } from '../design-system'
import type { InvoiceReminder } from '../types'
import './ReminderPanel.css'

interface ReminderPanelProps {
  invoiceId: number
  invoiceStatus: string
}

const METHOD_LABELS: Record<string, string> = {
  email: 'Email',
  sms: 'SMS',
  phone: 'Phone',
  manual: 'Manual',
}

export default function ReminderPanel({ invoiceId, invoiceStatus }: ReminderPanelProps) {
  const [reminders, setReminders] = useState<InvoiceReminder[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [method, setMethod] = useState<InvoiceReminder['method']>('email')
  const [notes, setNotes] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const list = await invoke<InvoiceReminder[]>('list_invoice_reminders', { invoiceId })
      setReminders(list)
    } catch (e) {
      console.error('Failed to load reminders:', e)
    } finally {
      setIsLoading(false)
    }
  }, [invoiceId])

  useEffect(() => { load() }, [load])

  const isPaid = invoiceStatus === 'paid' || invoiceStatus === 'voided'

  const handleLog = async () => {
    if (isSaving) return
    setError(null)
    setIsSaving(true)
    try {
      await invoke('log_invoice_reminder', { invoiceId, method, notes: notes.trim() })
      setNotes('')
      load()
    } catch (e) {
      setError(`Failed to log reminder: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  if (isLoading) return <div className="reminder-loading">Loading...</div>

  return (
    <div className="reminder-panel">
      <div className="reminder-header">
        <h4>Reminders</h4>
        <span className="reminder-count">{reminders.length} logged</span>
      </div>

      {isPaid ? (
        <div className="reminder-paid-notice">Invoice is {invoiceStatus} — no reminders needed.</div>
      ) : (
        <div className="reminder-log-form">
          <Select
            value={method}
            onChange={(e) => setMethod(e.target.value as InvoiceReminder['method'])}
            options={[
              { value: 'email', label: 'Email' },
              { value: 'sms', label: 'SMS' },
              { value: 'phone', label: 'Phone' },
              { value: 'manual', label: 'Manual' },
            ]}
          />
          <input
            className="reminder-notes-input"
            type="text"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder="Optional note (e.g. left voicemail)"
            maxLength={200}
            onKeyDown={(e) => { if (e.key === 'Enter') handleLog() }}
          />
          <Button variant="secondary" size="sm" onClick={handleLog} disabled={isSaving}>
            {isSaving ? '...' : 'Log Reminder'}
          </Button>
        </div>
      )}

      {error && <div className="reminder-error">{error}</div>}

      {reminders.length > 0 ? (
        <div className="reminder-list">
          {reminders.map((r) => (
            <div key={r.id} className="reminder-row">
              <span className="reminder-method">{METHOD_LABELS[r.method] ?? r.method}</span>
              <span className="reminder-date">{r.created_at.split(' ')[0]}</span>
              {r.notes && <span className="reminder-notes">{r.notes}</span>}
            </div>
          ))}
        </div>
      ) : (
        <div className="reminder-empty">No reminders logged yet.</div>
      )}
    </div>
  )
}
