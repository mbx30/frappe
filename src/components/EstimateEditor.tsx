import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Card } from '../design-system'
import type { Estimate, EstimateData, EstimateLineItem } from '../types'
import './EstimateEditor.css'

interface EstimateEditorProps {
  estimateId?: number
  onSave: () => void
  onCancel: () => void
}

const generateEstimateNumber = () => {
  return `EST-${Date.now().toString().slice(-8)}`
}

export default function EstimateEditor({ estimateId, onSave, onCancel }: EstimateEditorProps) {
  const [estimateData, setEstimateData] = useState<EstimateData | null>(null)
  const [isLoading, setIsLoading] = useState(!!estimateId)
  const [isSaving, setIsSaving] = useState(false)
  const [taxRate, setTaxRate] = useState(0)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (estimateId) {
      loadEstimate()
    } else {
      initializeNewEstimate()
    }
  }, [estimateId])

  const loadEstimate = async () => {
    if (!estimateId) return
    try {
      const data = await invoke<EstimateData>('get_estimate', { id: estimateId })
      setEstimateData(data)
      setTaxRate(data.estimate.tax_rate)
    } catch (e) {
      console.error('Failed to load estimate:', e)
    } finally {
      setIsLoading(false)
    }
  }

  const initializeNewEstimate = () => {
    const today = new Date().toISOString().split('T')[0]
    const validUntil = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
    setEstimateData({
      estimate: {
        id: 0,
        estimate_number: generateEstimateNumber(),
        client_id: null,
        status: 'draft',
        valid_until: validUntil,
        subtotal: 0,
        tax_rate: 0,
        tax_amount: 0,
        total: 0,
        currency: 'USD',
        notes: '',
        artwork_requirements: '',
        converted_order_id: null,
        created_at: today,
        updated_at: today,
      },
      line_items: [],
    })
    setTaxRate(0)
    setIsLoading(false)
  }

  if (isLoading || !estimateData) {
    return <div className="estimate-editor-loading">Loading...</div>
  }

  const { estimate, line_items } = estimateData

  const handleAddLineItem = (category: EstimateLineItem['category']) => {
    const newItem: EstimateLineItem = {
      id: 0,
      estimate_id: estimate.id || 0,
      description: '',
      category,
      quantity: 1,
      unit_price: 0,
      sort_order: line_items.length,
    }
    setEstimateData({
      ...estimateData,
      line_items: [...line_items, newItem],
    })
  }

  const handleUpdateLineItem = (index: number, updates: Partial<EstimateLineItem>) => {
    const updated = [...line_items]
    updated[index] = { ...updated[index], ...updates }
    setEstimateData({ ...estimateData, line_items: updated })
  }

  const handleRemoveLineItem = (index: number) => {
    setEstimateData({
      ...estimateData,
      line_items: line_items.filter((_, i) => i !== index),
    })
  }

  const calculateTotals = () => {
    const subtotal = line_items.reduce((sum, item) => sum + item.quantity * item.unit_price, 0)
    const tax = subtotal * (taxRate / 100)
    const total = subtotal + tax
    return { subtotal, tax, total }
  }

  const validate = (): string | null => {
    if (!estimateData) return 'No estimate loaded'
    if (!estimateData.estimate.estimate_number.trim()) return 'Estimate number is required'
    if (!estimateData.estimate.valid_until) return 'Valid until date is required'
    if (taxRate < 0 || taxRate > 100) return 'Tax rate must be between 0% and 100%'
    for (const item of estimateData.line_items) {
      if (!item.description.trim()) return 'All line items must have a description'
      if (item.quantity <= 0) return 'Line item quantities must be greater than zero'
      if (item.unit_price < 0) return 'Line item prices cannot be negative'
    }
    return null
  }

  const handleSave = async () => {
    if (!estimateData || isSaving) return
    const validationError = validate()
    if (validationError) { setError(validationError); return }
    setError(null)
    setIsSaving(true)
    try {
      const { subtotal, tax, total } = calculateTotals()

      if (estimate.id === 0) {
        // Create new estimate
        const newEstimate = await invoke<Estimate>('create_estimate', {
          estimate_number: estimate.estimate_number,
          valid_until: estimate.valid_until,
        })

        // Add line items
        for (const item of line_items) {
          await invoke('add_estimate_line_item', {
            estimate_id: newEstimate.id,
            description: item.description,
            category: item.category,
            quantity: item.quantity,
            unit_price: item.unit_price,
          })
        }

        // Update with totals
        await invoke('update_estimate', {
          id: newEstimate.id,
          status: estimate.status,
          subtotal,
          tax_rate: taxRate,
          tax_amount: tax,
          total,
          notes: estimate.notes,
          artwork_requirements: estimate.artwork_requirements,
        })
      } else {
        // Replace line items then update totals/metadata
        await invoke('replace_estimate_line_items', {
          estimateId: estimate.id,
          items: line_items.map((item) => ({
            description: item.description.trim(),
            category: item.category,
            quantity: item.quantity,
            unit_price: item.unit_price,
          })),
        })
        await invoke('update_estimate', {
          id: estimate.id,
          status: estimate.status,
          subtotal,
          tax_rate: taxRate,
          tax_amount: tax,
          total,
          notes: estimate.notes,
          artwork_requirements: estimate.artwork_requirements,
        })
      }

      onSave()
    } catch (e) {
      console.error('Failed to save estimate:', e)
      setError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const { subtotal, tax, total } = calculateTotals()

  return (
    <div className="estimate-editor">
      <div className="editor-header">
        <h2>{estimate.id === 0 ? 'New Estimate' : estimate.estimate_number}</h2>
        <div className="header-actions">
          <Button variant="secondary" onClick={onCancel} disabled={isSaving}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save Estimate'}
          </Button>
        </div>
      </div>

      {error && <div className="editor-error">{error}</div>}

      <div className="editor-grid">
        {/* Left column: Estimate details */}
        <div className="editor-section">
          <Card>
            <div className="card-title">Estimate Details</div>

            <div className="form-group">
              <label>Estimate Number</label>
              <Input
                value={estimate.estimate_number}
                onChange={(e) =>
                  setEstimateData({
                    ...estimateData,
                    estimate: { ...estimate, estimate_number: e.target.value },
                  })
                }
                disabled={estimate.id !== 0}
              />
            </div>

            <div className="form-row">
              <div className="form-group">
                <label>Valid Until</label>
                <Input
                  type="date"
                  value={estimate.valid_until}
                  onChange={(e) =>
                    setEstimateData({
                      ...estimateData,
                      estimate: { ...estimate, valid_until: e.target.value },
                    })
                  }
                />
              </div>
              <div className="form-group">
                <label>Status</label>
                <Select
                  value={estimate.status}
                  onChange={(e) =>
                    setEstimateData({
                      ...estimateData,
                      estimate: { ...estimate, status: e.target.value as Estimate['status'] },
                    })
                  }
                  options={[
                    { value: 'draft', label: 'Draft' },
                    { value: 'sent', label: 'Sent' },
                    { value: 'approved', label: 'Approved' },
                    { value: 'rejected', label: 'Rejected' },
                    { value: 'converted', label: 'Converted to Order' },
                  ]}
                />
              </div>
            </div>
          </Card>

          {/* Notes section */}
          <Card>
            <div className="card-title">Notes & Requirements</div>

            <div className="form-group">
              <label>Internal Notes</label>
              <textarea
                value={estimate.notes}
                onChange={(e) =>
                  setEstimateData({
                    ...estimateData,
                    estimate: { ...estimate, notes: e.target.value },
                  })
                }
                placeholder="For your reference only"
                className="textarea"
                rows={3}
              />
            </div>

            <div className="form-group">
              <label>Artwork Requirements</label>
              <textarea
                value={estimate.artwork_requirements}
                onChange={(e) =>
                  setEstimateData({
                    ...estimateData,
                    estimate: { ...estimate, artwork_requirements: e.target.value },
                  })
                }
                placeholder="Tell customer what artwork you need"
                className="textarea"
                rows={3}
              />
            </div>
          </Card>
        </div>

        {/* Right column: Line items & summary */}
        <div className="editor-section">
          <Card>
            <div className="card-title">Line Items</div>

            {/* Items by category */}
            {(['labor', 'materials', 'inventory', 'finishing'] as const).map((category) => {
              const categoryItems = line_items.filter((item) => item.category === category)
              return (
                <div key={category} className="category-section">
                  <div className="category-header">
                    <h4>{category.charAt(0).toUpperCase() + category.slice(1)}</h4>
                    <span className="item-count">({categoryItems.length})</span>
                  </div>

                  {categoryItems.map((item, idx) => {
                    const actualIndex = line_items.indexOf(item)
                    return (
                      <div key={actualIndex} className="line-item">
                        <Input
                          placeholder="Description"
                          value={item.description}
                          onChange={(e) =>
                            handleUpdateLineItem(actualIndex, { description: e.target.value })
                          }
                          maxLength={200}
                        />
                        <div className="line-item-row">
                          <Input
                            type="number"
                            placeholder="Qty"
                            value={item.quantity}
                            onChange={(e) =>
                              handleUpdateLineItem(actualIndex, {
                                quantity: Math.max(0.001, parseFloat(e.target.value) || 0),
                              })
                            }
                            inputMode="decimal"
                            min="0.001"
                          />
                          <Input
                            type="number"
                            placeholder="Unit Price"
                            value={item.unit_price}
                            onChange={(e) =>
                              handleUpdateLineItem(actualIndex, {
                                unit_price: Math.max(0, parseFloat(e.target.value) || 0),
                              })
                            }
                            inputMode="decimal"
                            min="0"
                          />
                          <div className="line-item-total">
                            ${(item.quantity * item.unit_price).toFixed(2)}
                          </div>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleRemoveLineItem(actualIndex)}
                          >
                            Remove
                          </Button>
                        </div>
                      </div>
                    )
                  })}

                  <Button
                    variant="secondary"
                    size="sm"
                    fullWidth
                    onClick={() => handleAddLineItem(category)}
                    style={{ marginBottom: '12px' }}
                  >
                    + Add {category}
                  </Button>
                </div>
              )
            })}
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
          </Card>
        </div>
      </div>
    </div>
  )
}
