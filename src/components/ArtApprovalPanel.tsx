import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Badge, Card } from '../design-system'
import type { ArtApproval } from '../types'
import './ArtApprovalPanel.css'

interface ArtApprovalPanelProps {
  orderId: number
  orderNumber: string
}

const statusColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  pending: 'warning',
  approved: 'success',
  changes_requested: 'danger',
}

const statusLabels: Record<string, string> = {
  pending: 'Awaiting Response',
  approved: 'Approved',
  changes_requested: 'Changes Requested',
}

export default function ArtApprovalPanel({ orderId, orderNumber }: ArtApprovalPanelProps) {
  const [approvals, setApprovals] = useState<ArtApproval[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [showNewForm, setShowNewForm] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [newForm, setNewForm] = useState({
    filePath: '',
    staffNotes: '',
    followUpHours: 24,
  })

  const load = useCallback(async () => {
    try {
      const list = await invoke<ArtApproval[]>('get_art_approvals_for_order', { orderId })
      setApprovals(list)
      setError(null)
    } catch (e) {
      console.error('Failed to load art approvals:', e)
      setError(`Failed to load approvals: ${e}`)
    } finally {
      setIsLoading(false)
    }
  }, [orderId])

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { load() }, [load])

  const handleSubmit = async () => {
    if (isSaving) return
    if (!newForm.filePath.trim() && !newForm.staffNotes.trim()) {
      setError('Add a file path or staff notes for this proof submission.')
      return
    }
    if (newForm.followUpHours < 1 || newForm.followUpHours > 720) {
      setError('Follow-up interval must be between 1 and 720 hours.')
      return
    }
    setError(null)
    setIsSaving(true)
    try {
      await invoke('create_art_approval', {
        orderId,
        filePath: newForm.filePath.trim(),
        staffNotes: newForm.staffNotes.trim(),
        followUpHours: newForm.followUpHours,
      })
      setNewForm({ filePath: '', staffNotes: '', followUpHours: 24 })
      setShowNewForm(false)
      load()
    } catch (e) {
      setError(`Failed to submit proof: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const handleFollowUp = async (id: number) => {
    try {
      await invoke('increment_art_approval_follow_up', { id })
      // load() has internal try/catch, but attach a catch here too so any
      // rejection from the unawaited promise is not silently swallowed and
      // we are not left with a state update on an unmounted component.
      load().catch((e) => console.error('Failed to reload approvals after follow-up:', e))
    } catch (e) {
      console.error('Failed to record follow-up:', e)
    }
  }

  if (isLoading) {
    return <div className="art-panel-loading">Loading approvals...</div>
  }

  const latestApproval = approvals[0]
  const pendingApproval = approvals.find((a) => a.status === 'pending')

  return (
    <div className="art-approval-panel">
      <div className="art-panel-header">
        <h4>Art Approvals</h4>
        {pendingApproval ? (
          <span className="art-panel-waiting">
            Waiting for customer response
          </span>
        ) : (
          <Button
            variant="secondary"
            size="sm"
            onClick={() => { setShowNewForm(true); setError(null) }}
          >
            + Submit Proof
          </Button>
        )}
      </div>

      {error && <div className="art-panel-error">{error}</div>}

      {showNewForm && (
        <Card className="art-new-form">
          <div className="card-title">
            Submit Proof — {orderNumber} v{(latestApproval?.version ?? 0) + 1}
          </div>

          <div className="form-group">
            <label>File Path / Reference</label>
            <input
              className="art-input"
              type="text"
              value={newForm.filePath}
              onChange={(e) => setNewForm((f) => ({ ...f, filePath: e.target.value }))}
              placeholder="e.g. /proofs/order-123-v1.pdf or shared drive link"
              maxLength={500}
            />
          </div>

          <div className="form-group">
            <label>Notes for Customer</label>
            <textarea
              className="art-textarea"
              value={newForm.staffNotes}
              onChange={(e) => setNewForm((f) => ({ ...f, staffNotes: e.target.value }))}
              placeholder="Describe the proof, any areas needing attention, or instructions..."
              rows={3}
              maxLength={1000}
            />
          </div>

          <div className="form-group">
            <label>Follow-up After (hours)</label>
            <input
              className="art-input art-input--narrow"
              type="number"
              value={newForm.followUpHours}
              onChange={(e) =>
                setNewForm((f) => ({
                  ...f,
                  followUpHours: Math.min(720, Math.max(1, parseInt(e.target.value) || 24)),
                }))
              }
              min="1"
              max="720"
            />
            <span className="field-hint">
              If no response, follow up after this many hours
            </span>
          </div>

          <div className="form-actions">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => { setShowNewForm(false); setError(null) }}
              disabled={isSaving}
            >
              Cancel
            </Button>
            <Button variant="primary" size="sm" onClick={handleSubmit} disabled={isSaving}>
              {isSaving ? 'Submitting...' : 'Submit Proof'}
            </Button>
          </div>
        </Card>
      )}

      {approvals.length === 0 && !showNewForm ? (
        <div className="art-panel-empty">
          No proofs submitted yet. Submit a proof to start the approval process.
        </div>
      ) : (
        <div className="art-versions">
          {approvals.map((approval) => (
            <div key={approval.id} className={`art-version art-version--${approval.status}`}>
              <div className="version-header">
                <span className="version-label">v{approval.version}</span>
                <Badge
                  tone={statusColors[approval.status]}
                >
                  {statusLabels[approval.status] || approval.status}
                </Badge>
                <span className="version-date">{approval.submitted_at.split(' ')[0]}</span>
              </div>

              {approval.file_path && (
                <div className="version-file">
                  <span className="file-icon">📄</span>
                  <span className="file-path">{approval.file_path}</span>
                </div>
              )}

              {approval.staff_notes && (
                <div className="version-notes">
                  <span className="notes-label">Staff notes:</span> {approval.staff_notes}
                </div>
              )}

              {approval.customer_notes && (
                <div className="version-notes version-notes--customer">
                  <span className="notes-label">Customer response:</span> {approval.customer_notes}
                </div>
              )}

              {approval.status === 'pending' && (
                <div className="version-actions">
                  <div className="follow-up-info">
                    Follow-up every {approval.follow_up_hours}h
                    {approval.follow_up_count > 0 && (
                      <span className="follow-up-count">
                        ({approval.follow_up_count} sent)
                      </span>
                    )}
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => handleFollowUp(approval.id)}
                  >
                    Log Follow-up
                  </Button>
                </div>
              )}

              {approval.responded_at && (
                <div className="version-responded">
                  Responded: {approval.responded_at.split(' ')[0]}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
