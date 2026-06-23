import { useState, useEffect, memo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card, Badge } from '../design-system'
import type { InventoryItem, InventoryAlert } from '../types'
import './InventoryList.css'

interface InventoryListProps {
  onCreateNew?: () => void
  onEditItem?: (id: number) => void
}

export default memo(function InventoryList({ onCreateNew, onEditItem }: InventoryListProps) {
  const [items, setItems] = useState<InventoryItem[]>([])
  const [alerts, setAlerts] = useState<InventoryAlert[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)

  async function loadInventory() {
    setIsLoading(true)
    setLoadError(null)
    try {
      const [itemsResult, alertsResult] = await Promise.all([
        invoke<InventoryItem[]>('list_inventory_items'),
        invoke<InventoryAlert[]>('get_low_stock_alerts'),
      ])
      setItems(itemsResult)
      setAlerts(alertsResult)
    } catch (e) {
      console.error('Failed to load inventory:', e)
      setLoadError(String(e))
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadInventory()
  }, [])

  const handleAcknowledgeAlert = async (alertId: number) => {
    try {
      await invoke('acknowledge_alert', { alertId })
      setAlerts(alerts.filter((a) => a.id !== alertId))
    } catch (e) {
      console.error('Failed to acknowledge alert:', e)
    }
  }

  const getStockStatus = (item: InventoryItem): 'critical' | 'low' | 'normal' => {
    if (item.alert_type === 'quantity') {
      if (item.quantity <= item.alert_threshold) return 'critical'
      if (item.quantity <= item.reorder_level) return 'low'
    } else if (item.alert_type === 'percentage') {
      if (item.reorder_level > 0) {
        const percentage = (item.quantity / item.reorder_level) * 100
        if (percentage <= item.alert_threshold) return 'critical'
        if (percentage <= item.alert_threshold * 1.5) return 'low'
      }
    }
    return 'normal'
  }

  const getStatusBadge = (status: string): 'success' | 'warning' | 'danger' | 'info' => {
    switch (status) {
      case 'critical':
        return 'danger'
      case 'low':
        return 'warning'
      default:
        return 'success'
    }
  }

  if (isLoading) {
    return (
      <div className="inventory-container">
        <div className="loading">Loading inventory...</div>
      </div>
    )
  }

  if (loadError) {
    return (
      <div className="inventory-container">
        <div className="inventory-header">
          <h2>Inventory</h2>
        </div>
        <Card className="empty-state">
          <div className="empty-content">
            <h3>Failed to load inventory</h3>
            <p>{loadError}</p>
            <Button variant="primary" onClick={loadInventory} style={{ marginTop: '16px' }}>
              Retry
            </Button>
          </div>
        </Card>
      </div>
    )
  }

  return (
    <div className="inventory-container">
      <div className="inventory-header">
        <div>
          <h2>Inventory</h2>
          <p className="subtitle">{items.length} items in stock</p>
        </div>
        <Button variant="primary" onClick={() => onCreateNew?.()}>
          + Add Item
        </Button>
      </div>

      {/* Alerts section */}
      {alerts.length > 0 && (
        <Card className="alerts-section">
          <div className="section-title">🚨 Stock Alerts ({alerts.length})</div>
          <div className="alerts-list">
              {alerts.map((alert) => {
              const item = items.find((i) => i.id === alert.inventory_item_id)
              if (!item) return null
              return (
                <div key={alert.id} className="alert-item">
                  <div className="alert-info">
                    <div className="alert-title">
                      {item.material_type} - {item.size}
                    </div>
                    <div className="alert-detail">
                      Current: {item.quantity} {item.unit} | Threshold:{' '}
                      {alert.threshold}
                    </div>
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => handleAcknowledgeAlert(alert.id)}
                  >
                    Acknowledge
                  </Button>
                </div>
              )
            })}
          </div>
        </Card>
      )}

      {/* Inventory items */}
      {items.length === 0 ? (
        <Card className="empty-state">
          <div className="empty-content">
            <h3>No inventory items</h3>
            <p>Add your first inventory item to get started</p>
            <Button
              variant="primary"
              onClick={() => onCreateNew?.()}
              style={{ marginTop: '16px' }}
            >
              Add Inventory Item
            </Button>
          </div>
        </Card>
      ) : (
        <div className="inventory-grid">
          {items.map((item) => {
            const status = getStockStatus(item)
            const stockPercentage =
              item.reorder_level > 0
                ? (item.quantity / item.reorder_level) * 100
                : 100

            return (
              <Card key={item.id} className="inventory-card">
                <div className="card-header">
                  <div>
                    <div className="item-name">
                      {item.material_type} - {item.size}
                    </div>
                    {item.attributes && (
                      <div className="item-attributes">{item.attributes}</div>
                    )}
                  </div>
                  <Badge
                    tone={getStatusBadge(status)}
                  >
                    {status.toUpperCase()}
                  </Badge>
                </div>

                <div className="card-body">
                  <div className="stock-info">
                    <div className="stock-row">
                      <span className="label">In Stock:</span>
                      <span className="value">
                        {item.quantity} {item.unit}
                      </span>
                    </div>
                    <div className="stock-row">
                      <span className="label">Reorder Level:</span>
                      <span className="value">{item.reorder_level}</span>
                    </div>

                    {/* Stock bar */}
                    <div className="stock-bar">
                      <div
                        className="stock-bar-fill"
                        style={{
                          width: `${Math.min(stockPercentage, 100)}%`,
                          backgroundColor:
                            status === 'critical'
                              ? 'var(--danger)'
                              : status === 'low'
                                ? 'var(--warning)'
                                : 'var(--success)',
                        }}
                      />
                    </div>
                    <div className="stock-percentage">
                      {stockPercentage.toFixed(0)}% of reorder level
                    </div>
                  </div>

                  {item.alert_type === 'quantity' && (
                    <div className="alert-config">
                      Alert when: ≤ {item.alert_threshold} {item.unit}
                    </div>
                  )}
                  {item.alert_type === 'percentage' && (
                    <div className="alert-config">
                      Alert when: ≤ {item.alert_threshold}% of reorder level
                    </div>
                  )}

                  {item.last_restocked && (
                    <div className="last-restocked">
                      Last restocked:{' '}
                      {new Date(item.last_restocked).toLocaleDateString()}
                    </div>
                  )}
                </div>

                <div className="card-actions">
                  <Button
                    variant="secondary"
                    size="sm"
                    fullWidth
                    onClick={() => onEditItem?.(item.id)}
                  >
                    Edit
                  </Button>
                </div>
              </Card>
            )
          })}
        </div>
      )}
    </div>
  )
}
