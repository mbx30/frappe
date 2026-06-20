import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Card, Checkbox } from '../design-system'
import type { Order, OrderData } from '../types'
import './OrderDetail.css'

interface OrderDetailProps {
  orderId?: number
  onSave: () => void
  onCancel: () => void
}

const generateOrderNumber = () => {
  return `ORD-${Date.now().toString().slice(-8)}`
}

const statusTransitions: Record<string, string[]> = {
  prepress: ['production'],
  production: ['delivery'],
  delivery: ['completed'],
  completed: [],
}

export default function OrderDetail({ orderId, onSave, onCancel }: OrderDetailProps) {
  const [orderData, setOrderData] = useState<OrderData | null>(null)
  const [isLoading, setIsLoading] = useState(!!orderId)
  const [isSaving, setIsSaving] = useState(false)
  const [transitionNotes, setTransitionNotes] = useState('')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (orderId) {
      loadOrder()
    } else {
      initializeNewOrder()
    }
  }, [orderId])

  const loadOrder = async () => {
    if (!orderId) return
    try {
      const data = await invoke<OrderData>('get_order', { id: orderId })
      setOrderData(data)
    } catch (e) {
      console.error('Failed to load order:', e)
    } finally {
      setIsLoading(false)
    }
  }

  const initializeNewOrder = () => {
    const today = new Date().toISOString().split('T')[0]
    const dueDate = new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
    setOrderData({
      order: {
        id: 0,
        order_number: generateOrderNumber(),
        client_id: null,
        status: 'prepress',
        priority: 'normal',
        due_date: dueDate,
        description: '',
        artwork_notes: '',
        artwork_url: null,
        artwork_approved: false,
        deposit_requested: false,
        deposit_amount: 0,
        total_value: 0,
        created_at: today,
        updated_at: today,
      },
      status_history: [],
    })
    setIsLoading(false)
  }

  const handleStatusChange = async (newStatus: string) => {
    if (!orderData) return
    try {
      await invoke('update_order_status', {
        order_id: orderData.order.id,
        new_status: newStatus,
        notes: transitionNotes,
      })
      setTransitionNotes('')
      await loadOrder()
    } catch (e) {
      console.error('Failed to update status:', e)
      setError(`Status update failed: ${e}`)
    }
  }

  const validate = (): string | null => {
    if (!orderData) return 'No order loaded'
    const { order } = orderData
    if (!order.order_number.trim()) return 'Order number is required'
    if (!order.description.trim()) return 'Description is required'
    if (!order.due_date) return 'Due date is required'
    if (order.deposit_amount < 0) return 'Deposit amount cannot be negative'
    if (order.total_value < 0) return 'Total value cannot be negative'
    if (order.deposit_requested && order.deposit_amount > order.total_value && order.total_value > 0) {
      return 'Deposit amount cannot exceed the total value'
    }
    return null
  }

  const handleSave = async () => {
    if (!orderData || isSaving) return
    const validationError = validate()
    if (validationError) { setError(validationError); return }
    setError(null)
    setIsSaving(true)
    try {
      if (orderData.order.id === 0) {
        // Create new order
        const newOrder = await invoke<Order>('create_order', {
          order_number: orderData.order.order_number,
          due_date: orderData.order.due_date,
          description: orderData.order.description,
        })

        // Update with details
        await invoke('update_order', {
          id: newOrder.id,
          priority: orderData.order.priority,
          description: orderData.order.description,
          artwork_notes: orderData.order.artwork_notes,
          artwork_approved: orderData.order.artwork_approved,
          deposit_requested: orderData.order.deposit_requested,
          deposit_amount: orderData.order.deposit_amount,
          total_value: orderData.order.total_value,
        })
      } else {
        // Update existing order
        await invoke('update_order', {
          id: orderData.order.id,
          priority: orderData.order.priority,
          description: orderData.order.description,
          artwork_notes: orderData.order.artwork_notes,
          artwork_approved: orderData.order.artwork_approved,
          deposit_requested: orderData.order.deposit_requested,
          deposit_amount: orderData.order.deposit_amount,
          total_value: orderData.order.total_value,
        })
      }

      onSave()
    } catch (e) {
      console.error('Failed to save order:', e)
      setError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  if (isLoading || !orderData) {
    return <div className="order-detail-loading">Loading...</div>
  }

  const { order, status_history } = orderData
  const availableTransitions = statusTransitions[order.status] || []

  const statusLabels: Record<string, string> = {
    prepress: 'Pre-press',
    production: 'Production',
    delivery: 'Delivery',
    completed: 'Completed',
  }

  return (
    <div className="order-detail">
      <div className="detail-header">
        <div>
          <h2>{order.id === 0 ? 'New Order' : order.order_number}</h2>
          <p className="status-badge">{statusLabels[order.status]}</p>
        </div>
        <div className="header-actions">
          <Button variant="secondary" onClick={onCancel} disabled={isSaving}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save Order'}
          </Button>
        </div>
      </div>

      {error && <div className="editor-error">{error}</div>}

      <div className="detail-grid">
        {/* Left column: Order details */}
        <div className="detail-section">
          <Card>
            <div className="card-title">Order Information</div>

            <div className="form-group">
              <label>Order Number</label>
              <Input
                value={order.order_number}
                onChange={(e) =>
                  setOrderData({
                    ...orderData,
                    order: { ...order, order_number: e.target.value },
                  })
                }
                disabled={order.id !== 0}
              />
            </div>

            <div className="form-group">
              <label>Description</label>
              <textarea
                value={order.description}
                onChange={(e) =>
                  setOrderData({
                    ...orderData,
                    order: { ...order, description: e.target.value },
                  })
                }
                placeholder="Order details"
                className="textarea"
                rows={3}
              />
            </div>

            <div className="form-row">
              <div className="form-group">
                <label>Due Date</label>
                <Input
                  type="date"
                  value={order.due_date}
                  onChange={(e) =>
                    setOrderData({
                      ...orderData,
                      order: { ...order, due_date: e.target.value },
                    })
                  }
                />
              </div>
              <div className="form-group">
                <label>Priority</label>
                <Select
                  value={order.priority}
                  onChange={(e) =>
                    setOrderData({
                      ...orderData,
                      order: { ...order, priority: e.target.value as Order['priority'] },
                    })
                  }
                  options={[
                    { value: 'low', label: 'Low' },
                    { value: 'normal', label: 'Normal' },
                    { value: 'high', label: 'High' },
                    { value: 'urgent', label: 'Urgent' },
                  ]}
                />
              </div>
            </div>

            <div className="form-group">
              <label>Order Value</label>
              <Input
                type="number"
                value={order.total_value}
                onChange={(e) =>
                  setOrderData({
                    ...orderData,
                    order: { ...order, total_value: Math.max(0, parseFloat(e.target.value) || 0) },
                  })
                }
                inputMode="decimal"
                placeholder="0.00"
                min="0"
              />
            </div>
          </Card>

          {/* Artwork section */}
          <Card>
            <div className="card-title">Artwork</div>

            <div className="form-group">
              <label>Artwork Notes</label>
              <textarea
                value={order.artwork_notes}
                onChange={(e) =>
                  setOrderData({
                    ...orderData,
                    order: { ...order, artwork_notes: e.target.value },
                  })
                }
                placeholder="Notes for customer about artwork requirements"
                className="textarea"
                rows={2}
              />
            </div>

            <div className="checkbox-group">
              <Checkbox
                checked={order.artwork_approved}
                onChange={(e) =>
                  setOrderData({
                    ...orderData,
                    order: { ...order, artwork_approved: e.target.checked },
                  })
                }
                label="Artwork Approved"
              />
            </div>
          </Card>

          {/* Deposit section */}
          <Card>
            <div className="card-title">Deposit</div>

            <div className="checkbox-group">
              <Checkbox
                checked={order.deposit_requested}
                onChange={(e) =>
                  setOrderData({
                    ...orderData,
                    order: { ...order, deposit_requested: e.target.checked },
                  })
                }
                label="Deposit Requested"
              />
            </div>

            {order.deposit_requested && (
              <div className="form-group">
                <label>Deposit Amount</label>
                <Input
                  type="number"
                  value={order.deposit_amount}
                  onChange={(e) =>
                    setOrderData({
                      ...orderData,
                      order: { ...order, deposit_amount: Math.max(0, parseFloat(e.target.value) || 0) },
                    })
                  }
                  inputMode="decimal"
                  placeholder="0.00"
                  min="0"
                />
              </div>
            )}
          </Card>
        </div>

        {/* Right column: Status workflow & history */}
        <div className="detail-section">
          {/* Status workflow */}
          <Card>
            <div className="card-title">Status Workflow</div>

            <div className="status-flow">
              <div className="status-current">
                <p className="label">Current Status</p>
                <p className="status-name">{statusLabels[order.status]}</p>
              </div>

              {availableTransitions.length > 0 && (
                <div className="status-transition">
                  <p className="label">Move to:</p>
                  <div className="transition-options">
                    {availableTransitions.map((nextStatus) => (
                      <Button
                        key={nextStatus}
                        variant="secondary"
                        size="sm"
                        onClick={() => handleStatusChange(nextStatus)}
                        fullWidth
                      >
                        → {statusLabels[nextStatus]}
                      </Button>
                    ))}
                  </div>

                  <div className="form-group">
                    <label>Transition Notes</label>
                    <textarea
                      value={transitionNotes}
                      onChange={(e) => setTransitionNotes(e.target.value)}
                      placeholder="Add notes about this status change"
                      className="textarea"
                      rows={2}
                    />
                  </div>
                </div>
              )}

              {availableTransitions.length === 0 && order.status === 'completed' && (
                <p className="label completed">✓ Order Completed</p>
              )}
            </div>
          </Card>

          {/* Status history */}
          <Card>
            <div className="card-title">Status History</div>

            {status_history.length === 0 ? (
              <p className="empty-history">No status changes yet</p>
            ) : (
              <div className="history-list">
                {status_history.map((entry) => (
                  <div key={entry.id} className="history-entry">
                    <div className="entry-header">
                      <span className="transition">
                        {entry.previous_status} → {entry.new_status}
                      </span>
                      <span className="date">
                        {new Date(entry.created_at).toLocaleString()}
                      </span>
                    </div>
                    {entry.notes && <p className="notes">{entry.notes}</p>}
                  </div>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
    </div>
  )
}
