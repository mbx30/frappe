import { useState, useEffect, memo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card, Badge } from '../design-system'
import type { Estimate } from '../types'
import './EstimateList.css'

interface EstimateListProps {
  onCreateNew: () => void
  onSelectEstimate: (id: number) => void
}

const statusColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  draft: 'info',
  sent: 'info',
  approved: 'success',
  rejected: 'danger',
  converted: 'success',
}

export default memo(function EstimateList({ onCreateNew, onSelectEstimate }: EstimateListProps) {
  const [estimates, setEstimates] = useState<Estimate[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)

  async function loadEstimates() {
    setIsLoading(true)
    setLoadError(null)
    try {
      const result = await invoke<Estimate[]>('list_estimates')
      setEstimates(result)
    } catch (e) {
      console.error('Failed to load estimates:', e)
      setLoadError(String(e))
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadEstimates()
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

  const isExpired = (validUntil: string) => {
    const today = new Date().toISOString().split('T')[0]
    return validUntil < today
  }

  if (isLoading) {
    return (
      <div className="estimate-list-container">
        <div className="loading">Loading estimates...</div>
      </div>
    )
  }

  if (loadError) {
    return (
      <div className="estimate-list-container">
        <div className="estimate-header">
          <h2>Estimates</h2>
        </div>
        <Card className="empty-state">
          <div className="empty-content">
            <h3>Failed to load estimates</h3>
            <p>{loadError}</p>
            <Button variant="primary" onClick={loadEstimates} style={{ marginTop: '16px' }}>
              Retry
            </Button>
          </div>
        </Card>
      </div>
    )
  }

  return (
    <div className="estimate-list-container">
      <div className="estimate-header">
        <div>
          <h2>Estimates</h2>
          <p className="subtitle">{estimates.length} total</p>
        </div>
        <Button variant="primary" onClick={onCreateNew}>
          + New Estimate
        </Button>
      </div>

      {estimates.length === 0 ? (
        <Card className="empty-state">
          <div className="empty-content">
            <h3>No estimates yet</h3>
            <p>Create your first estimate to get started</p>
            <Button variant="primary" onClick={onCreateNew} style={{ marginTop: '16px' }}>
              Create Estimate
            </Button>
          </div>
        </Card>
      ) : (
        <div className="estimate-table">
          <div className="table-header">
            <div className="col-number">Estimate #</div>
            <div className="col-date">Valid Until</div>
            <div className="col-amount">Amount</div>
            <div className="col-status">Status</div>
            <div className="col-actions">Actions</div>
          </div>
          {estimates.map((estimate) => (
            <div key={estimate.id} className="table-row" onClick={() => onSelectEstimate(estimate.id)}>
              <div className="col-number">
                <span className="estimate-number">{estimate.estimate_number}</span>
              </div>
              <div className="col-date">
                <span className={isExpired(estimate.valid_until) ? 'expired' : ''}>
                  {formatDate(estimate.valid_until)}
                </span>
              </div>
              <div className="col-amount">{formatCurrency(estimate.total, estimate.currency)}</div>
              <div className="col-status">
                <Badge
                  tone={statusColors[estimate.status] || 'info'}
                >
                  {estimate.status}
                </Badge>
              </div>
              <div className="col-actions" onClick={(e) => e.stopPropagation()}>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onSelectEstimate(estimate.id)}
                >
                  View
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
})
