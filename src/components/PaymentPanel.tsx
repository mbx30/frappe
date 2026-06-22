import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Select, Card } from '../design-system'
import type { Payment } from '../types'
import './PaymentPanel.css'

interface PaymentPanelProps {
  invoiceId?: number
  orderId?: number
  totalDue: number
  onPaymentRecorded?: () => void
}

const METHOD_LABELS: Record<string, string> = {
  cash: 'Cash',
  check: 'Check',
  card: 'Card',
  bank_transfer: 'Bank Transfer',
  other: 'Other',
}

const emptyForm = {
  amount: '',
  payment_method: 'cash' as Payment['payment_method'],
  reference: '',
  notes: '',
}

export default function PaymentPanel({ invoiceId, orderId, totalDue, onPaymentRecorded }: PaymentPanelProps) {
  const [payments, setPayments] = useState<Payment[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)
  const [form, setForm] = useState(emptyForm)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [deleteError, setDeleteError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const list = await invoke<Payment[]>('list_payments', {
        invoiceId: invoiceId ?? null,
        orderId: orderId ?? null,
      })
      setPayments(list)
    } catch (e) {
      console.error('Failed to load payments:', e)
    } finally {
      setIsLoading(false)
    }
  }, [invoiceId, orderId])

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { load() }, [load])

  const totalPaid = payments.reduce((s, p) => s + p.amount, 0)
  const balance = totalDue - totalPaid

  const validate = (): string | null => {
    const amt = parseFloat(form.amount)
    if (isNaN(amt) || amt <= 0) return 'Enter a valid payment amount greater than $0'
    if (amt > balance + 0.01) return `Amount ($${amt.toFixed(2)}) exceeds balance due ($${balance.toFixed(2)})`
    if (form.payment_method === 'check' && !form.reference.trim()) return 'Check number is required for check payments'
    return null
  }

  const handleSave = async () => {
    if (isSaving) return
    const err = validate()
    if (err) { setError(err); return }
    setError(null)
    setIsSaving(true)
    try {
      await invoke('record_payment', {
        invoiceId: invoiceId ?? null,
        orderId: orderId ?? null,
        amount: parseFloat(form.amount),
        paymentMethod: form.payment_method,
        reference: form.reference.trim(),
        notes: form.notes.trim(),
      })
      setForm(emptyForm)
      setShowForm(false)
      await load()
      onPaymentRecorded?.()
    } catch (e) {
      setError(`Failed to record payment: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const handleDelete = async (id: number, amount: number) => {
    if (!confirm(`Delete $${amount.toFixed(2)} payment? This will update the balance.`)) return
    setDeleteError(null)
    try {
      await invoke('delete_payment', { id })
      await load()
      onPaymentRecorded?.()
    } catch (e) {
      setDeleteError(`Could not delete payment: ${e}`)
    }
  }

  if (isLoading) return <div className="payment-loading">Loading payments...</div>

  return (
    <div className="payment-panel">
      <div className="payment-header">
        <h4>Payments</h4>
        {balance > 0.005 && !showForm && (
          <Button variant="secondary" size="sm" onClick={() => { setShowForm(true); setError(null) }}>
            + Record Payment
          </Button>
        )}
        {balance <= 0.005 && (
          <span className="payment-paid-badge">Paid in full</span>
        )}
      </div>

      <div className="payment-summary">
        <div className="summary-row">
          <span>Total Due</span>
          <span>${totalDue.toFixed(2)}</span>
        </div>
        {totalPaid > 0 && (
          <div className="summary-row summary-paid">
            <span>Total Paid</span>
            <span>−${totalPaid.toFixed(2)}</span>
          </div>
        )}
        <div className={`summary-row summary-balance ${balance <= 0.005 ? 'summary-balance--zero' : ''}`}>
          <span>Balance Due</span>
          <span>${Math.max(0, balance).toFixed(2)}</span>
        </div>
      </div>

      {error && <div className="payment-error">{error}</div>}
      {deleteError && <div className="payment-error">{deleteError}</div>}

      {showForm && (
        <Card className="payment-form">
          <div className="card-title">Record Payment</div>

          <div className="form-group">
            <label>Amount *</label>
            <div className="amount-input-wrap">
              <span className="currency-prefix">$</span>
              <input
                className="payment-input payment-input--amount"
                type="number"
                step="0.01"
                min="0.01"
                max={balance.toFixed(2)}
                value={form.amount}
                onChange={(e) => setForm((f) => ({ ...f, amount: e.target.value }))}
                placeholder="0.00"
                autoFocus
              />
            </div>
            {balance > 0 && (
              <button
                className="fill-balance-btn"
                type="button"
                onClick={() => setForm((f) => ({ ...f, amount: balance.toFixed(2) }))}
              >
                Fill balance (${balance.toFixed(2)})
              </button>
            )}
          </div>

          <div className="form-group">
            <label>Method *</label>
            <Select
              value={form.payment_method}
              onChange={(e) => setForm((f) => ({ ...f, payment_method: e.target.value as Payment['payment_method'] }))}
              options={[
                { value: 'cash', label: 'Cash' },
                { value: 'check', label: 'Check' },
                { value: 'card', label: 'Card' },
                { value: 'bank_transfer', label: 'Bank Transfer' },
                { value: 'other', label: 'Other' },
              ]}
            />
          </div>

          {form.payment_method === 'check' && (
            <div className="form-group">
              <label>Check Number *</label>
              <input
                className="payment-input"
                type="text"
                value={form.reference}
                onChange={(e) => setForm((f) => ({ ...f, reference: e.target.value }))}
                placeholder="e.g. 1042"
                maxLength={50}
              />
            </div>
          )}

          {form.payment_method === 'card' && (
            <div className="form-group">
              <label>Card Last 4</label>
              <input
                className="payment-input payment-input--narrow"
                type="text"
                value={form.reference}
                onChange={(e) => setForm((f) => ({ ...f, reference: e.target.value }))}
                placeholder="1234"
                maxLength={4}
              />
            </div>
          )}

          <div className="form-group">
            <label>Notes</label>
            <input
              className="payment-input"
              type="text"
              value={form.notes}
              onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))}
              placeholder="Optional notes"
              maxLength={200}
            />
          </div>

          <div className="form-actions">
            <Button variant="secondary" size="sm" onClick={() => { setShowForm(false); setError(null) }} disabled={isSaving}>
              Cancel
            </Button>
            <Button variant="primary" size="sm" onClick={handleSave} disabled={isSaving}>
              {isSaving ? 'Saving...' : 'Record Payment'}
            </Button>
          </div>
        </Card>
      )}

      {payments.length > 0 && (
        <div className="payment-history">
          {payments.map((p) => (
            <div key={p.id} className="payment-row">
              <div className="payment-row-left">
                <span className="payment-method-badge">{METHOD_LABELS[p.payment_method] ?? p.payment_method}</span>
                <span className="payment-date">{p.recorded_at.split(' ')[0]}</span>
                {p.reference && <span className="payment-ref">#{p.reference}</span>}
                {p.notes && <span className="payment-notes">{p.notes}</span>}
              </div>
              <div className="payment-row-right">
                <span className="payment-amount">${p.amount.toFixed(2)}</span>
                <Button variant="ghost" size="sm" onClick={() => handleDelete(p.id, p.amount)}>
                  ×
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {payments.length === 0 && !showForm && (
        <div className="payment-empty">No payments recorded yet.</div>
      )}
    </div>
  )
}
