import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Card } from '../design-system'
import type { Invoice, InvoiceData, InvoiceLineItem } from '../types'
import { allowedInvoiceTransitions, isValidInvoiceTransition, invoiceStatusLabel } from '../types'
import PaymentPanel from './PaymentPanel'
import ReminderPanel from './ReminderPanel'
import './InvoiceEditor.css'

interface InvoiceEditorProps {
  invoiceId?: number
  onSave: () => void
  onCancel: () => void
}

const generateInvoiceNumber = () => {
  return `INV-${Date.now().toString().slice(-8)}`
}

export default function InvoiceEditor({ invoiceId, onSave, onCancel }: InvoiceEditorProps) {
  const [invoice, setInvoice] = useState<Invoice | null>(null)
  const [lineItems, setLineItems] = useState<InvoiceLineItem[]>([])
  const [isLoading, setIsLoading] = useState(!!invoiceId)
  const [isSaving, setIsSaving] = useState(false)
  const [taxRate, setTaxRate] = useState(0)
  const [error, setError] = useState<string | null>(null)

  const loadInvoice = useCallback(async () => {
    if (!invoiceId) return
    try {
      const data = await invoke<InvoiceData>('get_invoice', { id: invoiceId })
      setInvoice(data.invoice)
      setLineItems(data.line_items)
      setTaxRate(data.invoice.tax_rate)
    } catch (e) {
      console.error('Failed to load invoice:', e)
    } finally {
      setIsLoading(false)
    }
  }, [invoiceId])

  function initializeNewInvoice() {
    const today = new Date().toISOString().split('T')[0]
    const dueDate = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
    setInvoice({
      id: 0,
      invoice_number: generateInvoiceNumber(),
      client_id: null,
      status: 'draft',
      issue_date: today,
      due_date: dueDate,
      payment_terms: 'net-30',
      subtotal: 0,
      tax_rate: 0,
      tax_amount: 0,
      total: 0,
      currency: 'USD',
      internal_notes: '',
      customer_notes: '',
      qb_sync_status: 'not_synced',
      amount_paid: 0,
      created_at: today,
      updated_at: today,
    })
    setLineItems([])
    setTaxRate(0)
    setIsLoading(false)
  }

  useEffect(() => {
    /* eslint-disable react-hooks/set-state-in-effect */
    if (invoiceId) {
      loadInvoice()
    } else {
      initializeNewInvoice()
    }
    /* eslint-enable react-hooks/set-state-in-effect */
  }, [invoiceId, loadInvoice])

  const handleAddLineItem = () => {
    const newItem: InvoiceLineItem & { tempId?: string } = {
      id: 0,
      invoice_id: invoice?.id || 0,
      description: '',
      quantity: 1,
      unit_price: 0,
      sort_order: lineItems.length,
      tempId: `temp-${Date.now()}-${Math.random()}`,
    }
    setLineItems([...lineItems, newItem])
  }

  const handleUpdateLineItem = (index: number, updates: Partial<InvoiceLineItem>) => {
    const updated = [...lineItems]
    updated[index] = { ...updated[index], ...updates }
    setLineItems(updated)
  }

  const handleRemoveLineItem = (index: number) => {
    setLineItems(lineItems.filter((_, i) => i !== index))
  }

  const calculateTotals = () => {
    const subtotal = lineItems.reduce((sum, item) => sum + item.quantity * item.unit_price, 0)
    const tax = subtotal * (taxRate / 100)
    const total = subtotal + tax
    return { subtotal, tax, total }
  }

  const validate = (): string | null => {
    if (!invoice) return 'No invoice loaded'
    if (!invoice.invoice_number.trim()) return 'Invoice number is required'
    if (!invoice.due_date) return 'Due date is required'
    if (taxRate < 0 || taxRate > 100) return 'Tax rate must be between 0% and 100%'
    for (const item of lineItems) {
      if (!item.description.trim()) return 'All line items must have a description'
      if (item.quantity <= 0) return 'Line item quantities must be greater than zero'
      if (item.unit_price < 0) return 'Line item prices cannot be negative'
    }
    return null
  }

  const handleSave = async () => {
    if (!invoice || isSaving) return
    const validationError = validate()
    if (validationError) { setError(validationError); return }
    setError(null)
    setIsSaving(true)
    try {
      const { subtotal, tax, total } = calculateTotals()

      if (invoice.id === 0) {
        const newInvoice = await invoke<Invoice>('create_invoice', {
          invoiceNumber: invoice.invoice_number.trim(),
          dueDate: invoice.due_date,
          paymentTerms: invoice.payment_terms,
        })
        for (const item of lineItems) {
          await invoke('add_invoice_line_item', {
            invoiceId: newInvoice.id,
            description: item.description.trim(),
            quantity: item.quantity,
            unitPrice: item.unit_price,
          })
        }
        await invoke('update_invoice', {
          id: newInvoice.id,
          status: invoice.status,
          subtotal,
          taxRate: taxRate,
          taxAmount: tax,
          total,
          internalNotes: invoice.internal_notes.trim(),
          customerNotes: invoice.customer_notes.trim(),
        })
      } else {
        await invoke('replace_invoice_line_items', {
          invoiceId: invoice.id,
          items: lineItems.map((item) => ({
            description: item.description.trim(),
            quantity: item.quantity,
            unit_price: item.unit_price,
          })),
        })
        await invoke('update_invoice', {
          id: invoice.id,
          status: invoice.status,
          subtotal,
          taxRate: taxRate,
          taxAmount: tax,
          total,
          internalNotes: invoice.internal_notes.trim(),
          customerNotes: invoice.customer_notes.trim(),
        })
      }

      onSave()
    } catch (e) {
      console.error('Failed to save invoice:', e)
      setError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  if (isLoading || !invoice) {
    return <div className="invoice-editor-loading">Loading...</div>
  }

  const { subtotal, tax, total } = calculateTotals()

  return (
    <div className="invoice-editor">
      <div className="editor-header">
        <h2>{invoice.id === 0 ? 'New Invoice' : `Invoice ${invoice.invoice_number}`}</h2>
        <div className="header-actions">
          <Button variant="secondary" onClick={onCancel} disabled={isSaving}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save Invoice'}
          </Button>
        </div>
      </div>

      {error && <div className="editor-error">{error}</div>}

      <div className="editor-grid">
        {/* Left column: Invoice details */}
        <div className="editor-section">
          <Card>
            <div className="card-title">Invoice Details</div>

            <div className="form-group">
              <label>Invoice Number</label>
              <Input
                value={invoice.invoice_number}
                onChange={(e) => setInvoice({ ...invoice, invoice_number: e.target.value })}
                disabled={invoice.id !== 0}
                maxLength={50}
              />
            </div>

            <div className="form-row">
              <div className="form-group">
                <label>Issue Date</label>
                <Input
                  type="date"
                  value={invoice.issue_date}
                  onChange={(e) => setInvoice({ ...invoice, issue_date: e.target.value })}
                />
              </div>
              <div className="form-group">
                <label>Due Date</label>
                <Input
                  type="date"
                  value={invoice.due_date}
                  onChange={(e) => setInvoice({ ...invoice, due_date: e.target.value })}
                />
              </div>
            </div>

            <div className="form-group">
              <label>Payment Terms</label>
              <Select
                value={invoice.payment_terms}
                onChange={(e) => setInvoice({ ...invoice, payment_terms: e.target.value })}
                options={[
                  { value: 'net-15', label: 'Net 15' },
                  { value: 'net-30', label: 'Net 30' },
                  { value: 'net-60', label: 'Net 60' },
                  { value: 'due-on-receipt', label: 'Due on Receipt' },
                ]}
              />
            </div>

            <div className="form-group">
              <label>Status</label>
              <Select
                value={invoice.status}
                onChange={(e) => {
                  const next = e.target.value as Invoice['status']
                  if (isValidInvoiceTransition(invoice.status, next)) {
                    setInvoice({ ...invoice, status: next })
                  } else {
                    alert(`Invalid transition: ${invoice.status} → ${next}`)
                  }
                }}
                options={[
                  { value: invoice.status, label: invoiceStatusLabel(invoice.status) },
                  ...allowedInvoiceTransitions(invoice.status)
                    .filter((s) => s !== invoice.status)
                    .map((s) => ({ value: s, label: invoiceStatusLabel(s) })),
                ]}
              />
            </div>
          </Card>

          {/* Notes section */}
          <Card>
            <div className="card-title">Notes</div>

            <div className="form-group">
              <label>Internal Notes</label>
              <textarea
                value={invoice.internal_notes}
                onChange={(e) => setInvoice({ ...invoice, internal_notes: e.target.value })}
                placeholder="For your reference only"
                className="notes-textarea"
                rows={3}
              />
            </div>

            <div className="form-group">
              <label>Customer Notes</label>
              <textarea
                value={invoice.customer_notes}
                onChange={(e) => setInvoice({ ...invoice, customer_notes: e.target.value })}
                placeholder="Visible to customer"
                className="notes-textarea"
                rows={3}
              />
            </div>
          </Card>
        </div>

        {/* Right column: Line items & summary */}
        <div className="editor-section">
          <Card>
            <div className="card-title">Line Items</div>
            <div className="line-items-list">
              {lineItems.map((item, index) => {
                const key = (item as InvoiceLineItem & { tempId?: string }).tempId || `item-${item.id}`
                return (
                  <div key={key} className="line-item">
                    <Input
                      placeholder="Description"
                      value={item.description}
                      onChange={(e) => handleUpdateLineItem(index, { description: e.target.value })}
                      maxLength={200}
                    />
                    <div className="line-item-row">
                      <Input
                        type="number"
                        placeholder="Qty"
                        value={item.quantity}
                        onChange={(e) => handleUpdateLineItem(index, { quantity: parseFloat(e.target.value) || 0 })}
                        inputMode="decimal"
                        min="0"
                      />
                      <Input
                        type="number"
                        placeholder="Unit Price"
                        value={item.unit_price}
                        onChange={(e) => handleUpdateLineItem(index, { unit_price: Math.max(0, parseFloat(e.target.value) || 0) })}
                        inputMode="decimal"
                        min="0"
                      />
                      <div className="line-item-total">
                        ${(item.quantity * item.unit_price).toFixed(2)}
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleRemoveLineItem(index)}
                      >
                        Remove
                      </Button>
                    </div>
                  </div>
                )
              })}
            </div>

            <Button variant="secondary" fullWidth onClick={handleAddLineItem} style={{ marginTop: '12px' }}>
              + Add Line Item
            </Button>
          </Card>

          {/* Summary */}
          <Card className="summary-card">
            <div className="summary-row">
              <span>Subtotal</span>
              <span>${subtotal.toFixed(2)}</span>
            </div>

            <div className="form-group" style={{ margin: '12px 0' }}>
              <label>Tax Rate (%)</label>
              <Input
                type="number"
                value={taxRate}
                onChange={(e) => setTaxRate(Math.min(100, Math.max(0, parseFloat(e.target.value) || 0)))}
                placeholder="0"
                inputMode="decimal"
                min="0"
                max="100"
              />
            </div>

            <div className="summary-row">
              <span>Tax ({taxRate}%)</span>
              <span>${tax.toFixed(2)}</span>
            </div>

            <div className="summary-row total">
              <span>Total</span>
              <span>${total.toFixed(2)}</span>
            </div>

            {invoice.id !== 0 && invoice.amount_paid > 0 && (
              <>
                <div className="summary-row">
                  <span>Amount Paid</span>
                  <span style={{ color: 'var(--success)' }}>${invoice.amount_paid.toFixed(2)}</span>
                </div>
                <div className="summary-row">
                  <span>Balance Due</span>
                  <span style={{ fontWeight: 'bold' }}>${Math.max(0, total - invoice.amount_paid).toFixed(2)}</span>
                </div>
              </>
            )}
          </Card>

          {/* Payments */}
          {invoice.id !== 0 && invoice.total > 0 && (
            <Card>
              <PaymentPanel invoiceId={invoice.id} totalDue={total} onPaymentRecorded={loadInvoice} />
            </Card>
          )}

          {/* Reminders */}
          {invoice.id !== 0 && (
            <Card>
              <ReminderPanel invoiceId={invoice.id} invoiceStatus={invoice.status} />
            </Card>
          )}
        </div>
      </div>
    </div>
  )
}
