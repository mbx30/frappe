import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Card } from '../design-system'
import type { Order } from '../types'
import './FulfillmentPanel.css'

interface FulfillmentPanelProps {
  order: Order
  onSaved?: () => void
}

export default function FulfillmentPanel({ order, onSaved }: FulfillmentPanelProps) {
  const [form, setForm] = useState({
    fulfillment_method: order.fulfillment_method,
    tracking_number: order.tracking_number,
    tracking_carrier: order.tracking_carrier,
    ready_for_pickup: order.ready_for_pickup,
    shipped_at: order.shipped_at ?? '',
  })
  const [isDirty, setIsDirty] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Reset form state when the order prop changes (e.g. switching orders)
  useEffect(() => {
    /* eslint-disable react-hooks/set-state-in-effect */
    setForm({
      fulfillment_method: order.fulfillment_method,
      tracking_number: order.tracking_number,
      tracking_carrier: order.tracking_carrier,
      ready_for_pickup: order.ready_for_pickup,
      shipped_at: order.shipped_at ?? '',
    })
    setIsDirty(false)
    setError(null)
    /* eslint-enable react-hooks/set-state-in-effect */
  }, [order.id, order.fulfillment_method, order.tracking_number, order.tracking_carrier, order.ready_for_pickup, order.shipped_at])

  const set = (k: keyof typeof form) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    setForm((f) => ({ ...f, [k]: e.target.value }))
    setIsDirty(true)
  }

  const setMethod = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setForm((f) => ({ ...f, fulfillment_method: e.target.value as Order['fulfillment_method'] }))
    setIsDirty(true)
  }

  const validate = (): string | null => {
    if (form.fulfillment_method === 'ship') {
      if (form.tracking_number && !form.tracking_carrier.trim()) return 'Enter the carrier name when adding a tracking number'
    }
    return null
  }

  const handleSave = async () => {
    if (isSaving || !order.id) return
    const err = validate()
    if (err) { setError(err); return }
    setError(null)
    setIsSaving(true)
    try {
      await invoke('update_order_fulfillment', {
        id: order.id,
        fulfillmentMethod: form.fulfillment_method,
        trackingNumber: form.tracking_number.trim(),
        trackingCarrier: form.tracking_carrier.trim(),
        readyForPickup: form.fulfillment_method === 'pickup' ? form.ready_for_pickup : false,
        shippedAt: form.shipped_at.trim() || null,
      })
      setIsDirty(false)
      onSaved?.()
    } catch (e) {
      setError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const togglePickupReady = () => {
    setForm((f) => ({ ...f, ready_for_pickup: !f.ready_for_pickup }))
    setIsDirty(true)
  }

  return (
    <Card className="fulfillment-panel">
      <div className="fulfillment-header">
        <div className="card-title">Fulfillment</div>
        {isDirty && (
          <Button variant="primary" size="sm" onClick={handleSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save'}
          </Button>
        )}
      </div>

      {error && <div className="fulfillment-error">{error}</div>}

      <div className="fulfillment-body">
        <div className="form-group">
          <label>Method</label>
          <Select
            value={form.fulfillment_method}
            onChange={setMethod}
            options={[
              { value: 'pickup', label: 'Customer Pickup' },
              { value: 'ship', label: 'Ship / Courier' },
              { value: 'delivery', label: 'Local Delivery' },
            ]}
          />
        </div>

        {form.fulfillment_method === 'pickup' && (
          <div className="form-group">
            <label>Pickup Status</label>
            <div className="pickup-toggle">
              <button
                className={`pickup-btn ${form.ready_for_pickup ? 'pickup-btn--ready' : ''}`}
                type="button"
                onClick={togglePickupReady}
              >
                {form.ready_for_pickup ? '✓ Ready for Pickup' : 'Mark Ready for Pickup'}
              </button>
              {form.ready_for_pickup && (
                <span className="pickup-hint">Customer can be notified to pick up their order.</span>
              )}
            </div>
          </div>
        )}

        {(form.fulfillment_method === 'ship' || form.fulfillment_method === 'delivery') && (
          <>
            <div className="form-group">
              <label>Carrier</label>
              <Input
                value={form.tracking_carrier}
                onChange={set('tracking_carrier')}
                placeholder="e.g. UPS, FedEx, USPS, DHL"
                maxLength={50}
              />
            </div>

            <div className="form-group">
              <label>Tracking Number</label>
              <Input
                value={form.tracking_number}
                onChange={set('tracking_number')}
                placeholder="Tracking number"
                maxLength={100}
              />
            </div>

            <div className="form-group">
              <label>Shipped Date</label>
              <Input
                type="date"
                value={form.shipped_at}
                onChange={set('shipped_at')}
              />
            </div>
          </>
        )}
      </div>

      {/* Status summary */}
      <div className="fulfillment-status">
        {form.fulfillment_method === 'pickup' && form.ready_for_pickup && (
          <div className="status-badge status-badge--ready">Ready for pickup</div>
        )}
        {form.fulfillment_method === 'pickup' && !form.ready_for_pickup && (
          <div className="status-badge status-badge--pending">Awaiting pickup</div>
        )}
        {form.fulfillment_method === 'ship' && form.tracking_number && (
          <div className="status-badge status-badge--shipped">
            Shipped via {form.tracking_carrier || 'carrier'} · {form.tracking_number}
          </div>
        )}
        {form.fulfillment_method === 'ship' && !form.tracking_number && (
          <div className="status-badge status-badge--pending">Not yet shipped</div>
        )}
        {form.fulfillment_method === 'delivery' && form.shipped_at && (
          <div className="status-badge status-badge--shipped">Out for delivery</div>
        )}
        {form.fulfillment_method === 'delivery' && !form.shipped_at && (
          <div className="status-badge status-badge--pending">Delivery pending</div>
        )}
      </div>
    </Card>
  )
}
