import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card } from '../design-system'
import type { Invoice } from '../types'
import './QBSyncPanel.css'

const QB_STATUS_LABELS: Record<string, string> = {
  not_synced: 'Not Synced',
  synced: 'Synced',
  sync_error: 'Sync Error',
  pending: 'Pending',
}

const QB_STATUS_CLASS: Record<string, string> = {
  not_synced: 'qb-badge--not-synced',
  synced: 'qb-badge--synced',
  sync_error: 'qb-badge--error',
  pending: 'qb-badge--pending',
}

export default function QBSyncPanel() {
  const [invoices, setInvoices] = useState<Invoice[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [syncing, setSyncing] = useState<Set<number>>(new Set())
  const [errors, setErrors] = useState<Map<number, string>>(new Map())
  const [successIds, setSuccessIds] = useState<Set<number>>(new Set())
  const [syncingAll, setSyncingAll] = useState(false)

  const load = useCallback(async () => {
    try {
      const list = await invoke<Invoice[]>('list_invoices')
      setInvoices(list)
    } catch (e) {
      console.error('Failed to load invoices:', e)
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  const handleSync = async (invoice: Invoice) => {
    if (syncing.has(invoice.id)) return
    setSyncing((prev) => new Set(prev).add(invoice.id))
    setErrors((prev) => { const m = new Map(prev); m.delete(invoice.id); return m })
    setSuccessIds((prev) => { const s = new Set(prev); s.delete(invoice.id); return s })

    try {
      await invoke('update_invoice_qb_status', { id: invoice.id, status: 'synced' })
      setSuccessIds((prev) => new Set(prev).add(invoice.id))
      await load()
    } catch (e) {
      setErrors((prev) => new Map(prev).set(invoice.id, `Sync failed: ${e}`))
      await invoke('update_invoice_qb_status', { id: invoice.id, status: 'sync_error' }).catch(() => {})
      await load()
    } finally {
      setSyncing((prev) => { const s = new Set(prev); s.delete(invoice.id); return s })
    }
  }

  const handleSyncAll = async () => {
    if (syncingAll) return
    setSyncingAll(true)
    try {
      const unsynced = invoices.filter(
        (inv) => inv.qb_sync_status !== 'synced' && inv.status !== 'draft' && inv.status !== 'voided'
      )
      for (const inv of unsynced) {
        await handleSync(inv)
      }
    } finally {
      setSyncingAll(false)
    }
  }

  const unsyncedCount = invoices.filter(
    (inv) => inv.qb_sync_status !== 'synced' && inv.status !== 'draft' && inv.status !== 'voided'
  ).length

  if (isLoading) return <div className="qb-loading">Loading...</div>

  return (
    <div className="qb-panel">
      <div className="qb-header">
        <div>
          <h2>QuickBooks Sync</h2>
          <p className="qb-subtitle">
            Manage sync status for invoices. QuickBooks integration requires API keys to be configured.
          </p>
        </div>
        <div className="qb-header-actions">
          <div className="qb-connection-badge qb-connection-badge--stub">
            Not Connected (Stub)
          </div>
          {unsyncedCount > 0 && (
            <Button variant="primary" onClick={handleSyncAll} disabled={syncingAll}>
              {syncingAll ? 'Syncing...' : `Mark All Synced (${unsyncedCount})`}
            </Button>
          )}
        </div>
      </div>

      <Card>
        <div className="qb-notice">
          <strong>QuickBooks integration is not yet connected.</strong> When API keys are configured,
          this panel will push invoices to QuickBooks automatically. For now, you can manually mark
          invoices as synced to track which ones have been entered in QuickBooks.
        </div>
      </Card>

      {invoices.length === 0 ? (
        <div className="qb-empty">No invoices yet.</div>
      ) : (
        <div className="qb-table-wrap">
          <table className="qb-table">
            <thead>
              <tr>
                <th>Invoice</th>
                <th>Status</th>
                <th>Total</th>
                <th>QB Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {invoices.map((inv) => (
                <tr key={inv.id}>
                  <td className="qb-inv-number">{inv.invoice_number}</td>
                  <td><span className={`qb-status-chip qb-status-chip--${inv.status}`}>{inv.status}</span></td>
                  <td>${inv.total.toFixed(2)}</td>
                  <td>
                    <span className={`qb-badge ${QB_STATUS_CLASS[inv.qb_sync_status] ?? ''}`}>
                      {QB_STATUS_LABELS[inv.qb_sync_status] ?? inv.qb_sync_status}
                    </span>
                    {successIds.has(inv.id) && (
                      <span className="qb-just-synced"> ✓</span>
                    )}
                  </td>
                  <td>
                    {errors.get(inv.id) && (
                      <div className="qb-row-error">{errors.get(inv.id)}</div>
                    )}
                    {inv.status !== 'draft' && inv.status !== 'voided' && inv.qb_sync_status !== 'synced' && (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => handleSync(inv)}
                        disabled={syncing.has(inv.id)}
                      >
                        {syncing.has(inv.id) ? 'Syncing...' : 'Mark Synced'}
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
