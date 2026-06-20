import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select } from '../design-system'
import type { Payment } from '../types'
import './POSView.css'

interface SearchResult {
  type: 'invoice' | 'order'
  id: number
  number: string
  status: string
  total: number
  amount_paid?: number
  balance?: number
}

const METHOD_LABELS: Record<string, string> = {
  cash: 'Cash',
  check: 'Check',
  card: 'Card',
  bank_transfer: 'Bank Transfer',
  other: 'Other',
}

export default function POSView() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResult[]>([])
  const [selected, setSelected] = useState<SearchResult | null>(null)
  const [isSearching, setIsSearching] = useState(false)
  const [searchError, setSearchError] = useState<string | null>(null)

  const [payAmount, setPayAmount] = useState('')
  const [payMethod, setPayMethod] = useState<Payment['payment_method']>('cash')
  const [payRef, setPayRef] = useState('')
  const [payNotes, setPayNotes] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [payError, setPayError] = useState<string | null>(null)
  const [successMsg, setSuccessMsg] = useState<string | null>(null)

  const handleSearch = useCallback(async () => {
    const q = query.trim()
    if (!q) return
    setIsSearching(true)
    setSearchError(null)
    setResults([])
    setSelected(null)
    setSuccessMsg(null)
    try {
      const res = await invoke<SearchResult[]>('search_invoices_and_orders', { query: q })
      setResults(res)
      if (res.length === 0) setSearchError(`No invoices or orders found matching "${q}"`)
    } catch (e) {
      setSearchError(`Search failed: ${e}`)
    } finally {
      setIsSearching(false)
    }
  }, [query])

  const handleSelect = (r: SearchResult) => {
    setSelected(r)
    const balance = r.balance ?? r.total
    setPayAmount(balance.toFixed(2))
    setPayError(null)
    setSuccessMsg(null)
  }

  const validatePay = (): string | null => {
    const amt = parseFloat(payAmount)
    if (isNaN(amt) || amt <= 0) return 'Enter a valid amount'
    const balance = selected ? (selected.balance ?? selected.total) : 0
    if (amt > balance + 0.01) return `Amount exceeds balance ($${balance.toFixed(2)})`
    if (payMethod === 'check' && !payRef.trim()) return 'Check number required'
    return null
  }

  const handlePay = async () => {
    if (isSaving || !selected) return
    const err = validatePay()
    if (err) { setPayError(err); return }
    setPayError(null)
    setIsSaving(true)
    try {
      await invoke('record_payment', {
        invoiceId: selected.type === 'invoice' ? selected.id : null,
        orderId: selected.type === 'order' ? selected.id : null,
        amount: parseFloat(payAmount),
        paymentMethod: payMethod,
        reference: payRef.trim(),
        notes: payNotes.trim(),
      })
      const paid = parseFloat(payAmount)
      const balance = selected.balance ?? selected.total
      const remaining = Math.max(0, balance - paid)
      setSuccessMsg(`Payment of $${paid.toFixed(2)} recorded for ${selected.type} ${selected.number}. ${remaining > 0 ? `Remaining balance: $${remaining.toFixed(2)}` : 'Paid in full.'}`)
      setSelected(null)
      setResults([])
      setQuery('')
      setPayAmount('')
      setPayRef('')
      setPayNotes('')
      setPayMethod('cash')
    } catch (e) {
      setPayError(`Payment failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const balance = selected ? (selected.balance ?? selected.total) : 0

  return (
    <div className="pos-view">
      <div className="pos-header">
        <h2>Point of Sale</h2>
        <p className="pos-subtitle">Look up an order or invoice by number to record payment.</p>
      </div>

      {successMsg && (
        <div className="pos-success">
          {successMsg}
          <Button variant="secondary" size="sm" onClick={() => setSuccessMsg(null)}>Dismiss</Button>
        </div>
      )}

      <div className="pos-search-bar">
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Order or invoice number (e.g. ORD-001, INV-042)"
          onKeyDown={(e: React.KeyboardEvent) => { if (e.key === 'Enter') handleSearch() }}
        />
        <Button variant="primary" onClick={handleSearch} disabled={isSearching || !query.trim()}>
          {isSearching ? 'Searching...' : 'Search'}
        </Button>
      </div>

      {searchError && <div className="pos-error">{searchError}</div>}

      {results.length > 0 && !selected && (
        <div className="pos-results">
          {results.map((r) => (
            <button key={`${r.type}-${r.id}`} className="pos-result-row" onClick={() => handleSelect(r)}>
              <span className="pos-type-badge pos-type-badge--{r.type}">{r.type}</span>
              <span className="pos-result-number">{r.number}</span>
              <span className="pos-result-status">{r.status}</span>
              <span className="pos-result-balance">
                {r.type === 'invoice'
                  ? `$${(r.balance ?? r.total).toFixed(2)} due`
                  : `$${r.total.toFixed(2)}`}
              </span>
            </button>
          ))}
        </div>
      )}

      {selected && (
        <div className="pos-payment-box">
          <div className="pos-payment-header">
            <div>
              <div className="pos-payment-title">{selected.type === 'invoice' ? 'Invoice' : 'Order'} {selected.number}</div>
              <div className="pos-balance-due">Balance due: <strong>${balance.toFixed(2)}</strong></div>
            </div>
            <Button variant="ghost" size="sm" onClick={() => { setSelected(null); setResults([]) }}>
              ← Change
            </Button>
          </div>

          {payError && <div className="pos-error">{payError}</div>}

          <div className="pos-pay-form">
            <div className="form-group">
              <label>Amount</label>
              <div className="pos-amount-wrap">
                <span className="pos-currency">$</span>
                <input
                  className="pos-amount-input"
                  type="number"
                  step="0.01"
                  min="0.01"
                  value={payAmount}
                  onChange={(e) => setPayAmount(e.target.value)}
                  autoFocus
                />
              </div>
              {balance > 0 && parseFloat(payAmount) !== balance && (
                <button className="pos-fill-btn" type="button" onClick={() => setPayAmount(balance.toFixed(2))}>
                  Fill balance (${balance.toFixed(2)})
                </button>
              )}
            </div>

            <div className="form-group">
              <label>Method</label>
              <Select
                value={payMethod}
                onChange={(e) => setPayMethod(e.target.value as Payment['payment_method'])}
                options={Object.entries(METHOD_LABELS).map(([v, l]) => ({ value: v, label: l }))}
              />
            </div>

            {payMethod === 'check' && (
              <div className="form-group">
                <label>Check Number *</label>
                <Input value={payRef} onChange={(e) => setPayRef(e.target.value)} placeholder="e.g. 1042" maxLength={50} />
              </div>
            )}

            {payMethod === 'card' && (
              <div className="form-group">
                <label>Card Last 4</label>
                <Input value={payRef} onChange={(e) => setPayRef(e.target.value)} placeholder="1234" maxLength={4} />
              </div>
            )}

            <div className="form-group">
              <label>Notes</label>
              <Input value={payNotes} onChange={(e) => setPayNotes(e.target.value)} placeholder="Optional" maxLength={200} />
            </div>

            <Button variant="primary" onClick={handlePay} disabled={isSaving}>
              {isSaving ? 'Processing...' : `Collect $${parseFloat(payAmount || '0').toFixed(2)}`}
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
