import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Card } from '../design-system'
import type { Order, DepartmentNote } from '../types'
import './JobSpecsPanel.css'

interface JobSpecsPanelProps {
  order: Order
  onSaved?: () => void
}

const DEPARTMENTS = [
  { value: 'general', label: 'General' },
  { value: 'design', label: 'Design' },
  { value: 'prepress', label: 'Prepress' },
  { value: 'press', label: 'Press' },
  { value: 'finishing', label: 'Finishing' },
  { value: 'shipping', label: 'Shipping' },
]

export default function JobSpecsPanel({ order, onSaved }: JobSpecsPanelProps) {
  const [specs, setSpecs] = useState({
    print_type: order.print_type,
    paper_stock: order.paper_stock,
    ink_colors: order.ink_colors,
    finishing: order.finishing,
    quantity: String(order.quantity || ''),
    production_notes: order.production_notes,
    assigned_operator: order.assigned_operator,
  })
  const [isDirty, setIsDirty] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [saveError, setSpecsError] = useState<string | null>(null)

  const [notes, setNotes] = useState<DepartmentNote[]>([])
  const [newNote, setNewNote] = useState('')
  const [newDept, setNewDept] = useState<DepartmentNote['department']>('general')
  const [isAddingNote, setIsAddingNote] = useState(false)
  const [noteError, setNoteError] = useState<string | null>(null)

  const loadNotes = useCallback(async () => {
    if (!order.id) return
    try {
      const list = await invoke<DepartmentNote[]>('list_department_notes', { orderId: order.id })
      setNotes(list)
    } catch (e) {
      console.error('Failed to load department notes:', e)
    }
  }, [order.id])

  useEffect(() => { loadNotes() }, [loadNotes])

  const setSpec = (k: keyof typeof specs) => (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    setSpecs((s) => ({ ...s, [k]: e.target.value }))
    setIsDirty(true)
  }

  const handleSaveSpecs = async () => {
    if (isSaving || !order.id) return
    const qty = parseInt(specs.quantity) || 0
    if (qty < 0) { setSpecsError('Quantity cannot be negative'); return }
    setSpecsError(null)
    setIsSaving(true)
    try {
      await invoke('update_order_job_specs', {
        id: order.id,
        printType: specs.print_type.trim(),
        paperStock: specs.paper_stock.trim(),
        inkColors: specs.ink_colors.trim(),
        finishing: specs.finishing.trim(),
        quantity: qty,
        productionNotes: specs.production_notes.trim(),
        assignedOperator: specs.assigned_operator.trim(),
      })
      setIsDirty(false)
      onSaved?.()
    } catch (e) {
      setSpecsError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const handleAddNote = async () => {
    if (isAddingNote) return
    if (!newNote.trim()) { setNoteError('Note cannot be empty'); return }
    setNoteError(null)
    setIsAddingNote(true)
    try {
      await invoke('add_department_note', { orderId: order.id, note: newNote.trim(), department: newDept })
      setNewNote('')
      setNewDept('general')
      loadNotes()
    } catch (e) {
      setNoteError(`Failed to add note: ${e}`)
    } finally {
      setIsAddingNote(false)
    }
  }

  const handleDeleteNote = async (id: number) => {
    if (!confirm('Delete this note?')) return
    try {
      await invoke('delete_department_note', { id })
      loadNotes()
    } catch (e) {
      setNoteError(`Failed to delete note: ${e}`)
    }
  }

  return (
    <div className="job-specs-panel">
      <Card>
        <div className="specs-header">
          <div className="card-title">Job Specs</div>
          {isDirty && (
            <Button variant="primary" size="sm" onClick={handleSaveSpecs} disabled={isSaving}>
              {isSaving ? 'Saving...' : 'Save Specs'}
            </Button>
          )}
        </div>

        {saveError && <div className="specs-error">{saveError}</div>}

        <div className="specs-grid">
          <div className="form-group">
            <label>Print Type</label>
            <Input
              value={specs.print_type}
              onChange={setSpec('print_type')}
              placeholder="e.g. Digital, Offset, Wide format"
              maxLength={100}
            />
          </div>

          <div className="form-group">
            <label>Quantity</label>
            <Input
              type="number"
              value={specs.quantity}
              onChange={setSpec('quantity')}
              placeholder="0"
              min="0"
            />
          </div>

          <div className="form-group">
            <label>Paper / Stock</label>
            <Input
              value={specs.paper_stock}
              onChange={setSpec('paper_stock')}
              placeholder="e.g. 100lb Gloss Cover"
              maxLength={100}
            />
          </div>

          <div className="form-group">
            <label>Ink Colors</label>
            <Input
              value={specs.ink_colors}
              onChange={setSpec('ink_colors')}
              placeholder="e.g. 4/4 CMYK, 1/0 Black"
              maxLength={100}
            />
          </div>

          <div className="form-group specs-full">
            <label>Finishing</label>
            <Input
              value={specs.finishing}
              onChange={setSpec('finishing')}
              placeholder="e.g. Laminate, Fold, Score, Saddle stitch"
              maxLength={200}
            />
          </div>

          <div className="form-group">
            <label>Assigned Operator</label>
            <Input
              value={specs.assigned_operator}
              onChange={setSpec('assigned_operator')}
              placeholder="Operator name or initials"
              maxLength={100}
            />
          </div>

          <div className="form-group specs-full">
            <label>Production Notes</label>
            <textarea
              className="specs-textarea"
              value={specs.production_notes}
              onChange={setSpec('production_notes')}
              placeholder="Notes for press operators..."
              rows={3}
              maxLength={1000}
            />
          </div>
        </div>
      </Card>

      <Card>
        <div className="card-title">Department Notes</div>
        <p className="dept-note-hint">Internal staff notes — not visible to the customer.</p>

        {noteError && <div className="specs-error">{noteError}</div>}

        <div className="dept-note-form">
          <Select
            value={newDept}
            onChange={(e) => setNewDept(e.target.value as DepartmentNote['department'])}
            options={DEPARTMENTS}
          />
          <input
            className="dept-note-input"
            type="text"
            value={newNote}
            onChange={(e) => { setNewNote(e.target.value); setNoteError(null) }}
            placeholder="Add a note..."
            maxLength={500}
            onKeyDown={(e) => { if (e.key === 'Enter') handleAddNote() }}
          />
          <Button variant="secondary" size="sm" onClick={handleAddNote} disabled={isAddingNote || !newNote.trim()}>
            {isAddingNote ? '...' : 'Add'}
          </Button>
        </div>

        {notes.length > 0 ? (
          <div className="dept-notes-list">
            {notes.map((n) => (
              <div key={n.id} className="dept-note">
                <div className="dept-note-meta">
                  <span className="dept-badge">{n.department}</span>
                  <span className="dept-note-date">{n.created_at.split(' ')[0]}</span>
                </div>
                <div className="dept-note-body">
                  <span className="dept-note-text">{n.note}</span>
                  <Button variant="ghost" size="sm" onClick={() => handleDeleteNote(n.id)}>×</Button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="dept-notes-empty">No department notes yet.</p>
        )}
      </Card>
    </div>
  )
}
