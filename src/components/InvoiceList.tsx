import { useState, useEffect, memo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card, Badge } from '../design-system'
import type { Invoice } from '../types'
import './InvoiceList.css'

interface InvoiceListProps {
  onCreateNew: () => void
  onEditInvoice: (id: number) => void
}

const statusColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  draft: 'info',
  sent: 'info',
  'partially-paid': 'warning',
  paid: 'success',
  overdue: 'danger',
  voided: 'info',
}

export default memo(function InvoiceList({ onCreateNew, onEditInvoice }: InvoiceListProps) {
  const [invoices, setInvoices] = useState<Invoice[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)

  async function loadInvoices() {
    setIsLoading(true)
    setLoadError(null)
    try {
      const result = await invoke<Invoice[]>('list_invoices')
      setInvoices(result)
    } catch (e) {
      console.error('Failed to load invoices:', e)
      setLoadError(String(e))
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadInvoices()
  }, [])

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString()
  }

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
    }).format(amount)
  }

  if (isLoading) {
    return (
      <div className="invoice-list-container">
        <div className="loading">Loading invoices...</div>
      </div>
    )
  }

  if (loadError) {
    return (
      <div className="invoice-list-container">
        <div className="invoice-header">
          <h2>Invoices</h2>
        </div>
        <Card className="empty-state">
          <div className="empty-content">
            <h3>Failed to load invoices</h3>
            <p>{loadError}</p>
            <Button variant="primary" onClick={loadInvoices} style={{ marginTop: '16px' }}>
              Retry
            </Button>
          </div>
        </Card>
      </div>
    )
  }

  return (
    <div className="invoice-list-container">
      <div className="invoice-header">
        <div>
          <h2>Invoices</h2>
          <p className="subtitle">{invoices.length} total</p>
        </div>
        <Button variant="primary" onClick={onCreateNew}>
          + New Invoice
        </Button>
      </div>

      {invoices.length === 0 ? (
        <Card className="empty-state">
          <div className="empty-content">
            <h3>No invoices yet</h3>
            <p>Create your first invoice to get started</p>
            <Button variant="primary" onClick={onCreateNew} style={{ marginTop: '16px' }}>
              Create Invoice
            </Button>
          </div>
        </Card>
      ) : (
        <div className="invoice-table">
          <div className="table-header">
            <div className="col-number">Invoice #</div>
            <div className="col-date">Date</div>
            <div className="col-amount">Amount</div>
            <div className="col-status">Status</div>
            <div className="col-actions">Actions</div>
          </div>
          {invoices.map((invoice) => (
            <div key={invoice.id} className="table-row" onClick={() => onEditInvoice(invoice.id)}>
              <div className="col-number">
                <span className="invoice-number">{invoice.invoice_number}</span>
              </div>
              <div className="col-date">{formatDate(invoice.issue_date)}</div>
              <div className="col-amount">{formatCurrency(invoice.total, invoice.currency)}</div>
              <div className="col-status">
                <Badge
                  tone={statusColors[invoice.status] || 'info'}
                >
                  {invoice.status.replace('-', ' ')}
                </Badge>
              </div>
              <div className="col-actions" onClick={(e) => e.stopPropagation()}>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onEditInvoice(invoice.id)}
                >
                  Edit
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
