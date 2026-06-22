import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Card } from '../design-system'
import type { Client } from '../types'
import './ClientForm.css'

interface ClientFormProps {
  client?: Client
  onSave: () => void
  onCancel: () => void
}

const emptyForm = {
  name: '',
  company: '',
  email: '',
  phone: '',
  address: '',
  tags: '',
  status: 'active' as Client['status'],
  notes: '',
}

export default function ClientForm({ client, onSave, onCancel }: ClientFormProps) {
  const [form, setForm] = useState(emptyForm)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    /* eslint-disable react-hooks/set-state-in-effect */
    if (client) {
      setForm({
        name: client.name,
        company: client.company,
        email: client.email,
        phone: client.phone,
        address: client.address,
        tags: client.tags,
        status: client.status,
        notes: client.notes,
      })
    } else {
      setForm(emptyForm)
    }
    /* eslint-enable react-hooks/set-state-in-effect */
  }, [client])

  const validate = (): string | null => {
    if (!form.name.trim()) return 'Client name is required'
    if (form.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(form.email)) return 'Invalid email address'
    return null
  }

  const normalizeTags = (raw: string) =>
    raw.split(',').map((t) => t.trim()).filter(Boolean).join(', ')

  const handleSave = async () => {
    if (isSaving) return
    const err = validate()
    if (err) { setError(err); return }
    setError(null)
    setIsSaving(true)
    const cleanTags = normalizeTags(form.tags)
    try {
      if (client) {
        await invoke('update_client', {
          id: client.id,
          name: form.name.trim(),
          company: form.company.trim(),
          email: form.email.trim().toLowerCase(),
          phone: form.phone.trim(),
          address: form.address.trim(),
          tags: cleanTags,
          status: form.status,
          notes: form.notes.trim(),
        })
      } else {
        await invoke('create_client', {
          name: form.name.trim(),
          company: form.company.trim(),
          email: form.email.trim().toLowerCase(),
          phone: form.phone.trim(),
          address: form.address.trim(),
          tags: cleanTags,
        })
      }
      onSave()
    } catch (e) {
      setError(`Save failed: ${e}`)
    } finally {
      setIsSaving(false)
    }
  }

  const set = (field: keyof typeof form) => (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) =>
    setForm((f) => ({ ...f, [field]: e.target.value }))

  return (
    <div className="client-form">
      <div className="form-header">
        <h2>{client ? 'Edit Client' : 'New Client'}</h2>
        <div className="header-actions">
          <Button variant="secondary" onClick={onCancel} disabled={isSaving}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save Client'}
          </Button>
        </div>
      </div>

      {error && <div className="editor-error">{error}</div>}

      <div className="form-grid">
        <div className="form-col">
          <Card>
            <div className="card-title">Contact Details</div>

            <div className="form-group">
              <label>Name *</label>
              <Input
                value={form.name}
                onChange={set('name')}
                placeholder="Full name"
                maxLength={100}
              />
            </div>

            <div className="form-group">
              <label>Company</label>
              <Input
                value={form.company}
                onChange={set('company')}
                placeholder="Company or organization"
                maxLength={100}
              />
            </div>

            <div className="form-group">
              <label>Email</label>
              <Input
                type="email"
                value={form.email}
                onChange={set('email')}
                placeholder="email@example.com"
                maxLength={200}
              />
            </div>

            <div className="form-group">
              <label>Phone</label>
              <Input
                type="tel"
                value={form.phone}
                onChange={set('phone')}
                placeholder="Phone number"
                maxLength={50}
              />
            </div>

            <div className="form-group">
              <label>Address</label>
              <textarea
                value={form.address}
                onChange={set('address')}
                placeholder="Street, city, state, zip"
                className="textarea"
                rows={3}
                maxLength={300}
              />
            </div>
          </Card>
        </div>

        <div className="form-col">
          <Card>
            <div className="card-title">Classification</div>

            {client && (
              <div className="form-group">
                <label>Status</label>
                <Select
                  value={form.status}
                  onChange={set('status')}
                  options={[
                    { value: 'active', label: 'Active' },
                    { value: 'inactive', label: 'Inactive' },
                  ]}
                />
              </div>
            )}

            <div className="form-group">
              <label>Tags</label>
              <Input
                value={form.tags}
                onChange={set('tags')}
                placeholder="e.g. vip, retail, corporate (comma-separated)"
                maxLength={200}
              />
              <span className="field-hint">Separate multiple tags with commas</span>
            </div>
          </Card>

          <Card>
            <div className="card-title">Notes</div>
            <div className="form-group">
              <textarea
                value={form.notes}
                onChange={set('notes')}
                placeholder="Internal notes about this client..."
                className="textarea"
                rows={6}
                maxLength={2000}
              />
            </div>
          </Card>
        </div>
      </div>
    </div>
  )
}
