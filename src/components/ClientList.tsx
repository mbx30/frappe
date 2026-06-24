import { useState, useEffect, useCallback, memo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select, Badge } from '../design-system'
import type { Client } from '../types'
import { VirtualList } from './common/VirtualList'
import './ClientList.css'

interface ClientListProps {
  onSelectClient: (client: Client) => void
  onNewClient: () => void
}

const VIRTUAL_THRESHOLD = 200

const statusColors: Record<string, 'success' | 'info'> = {
  active: 'success',
  inactive: 'info',
}

export default memo(function ClientList({ onSelectClient, onNewClient }: ClientListProps) {
  const [clients, setClients] = useState<Client[]>([])
  const [search, setSearch] = useState('')
  const [statusFilter, setStatusFilter] = useState('')
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [deleteError, setDeleteError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setIsLoading(true)
    setLoadError(null)
    try {
      const list = await invoke<Client[]>('list_clients', {
        search: search.trim() || null,
        statusFilter: statusFilter || null,
      })
      setClients(list)
    } catch (e) {
      console.error('Failed to load clients:', e)
      setLoadError(String(e))
    } finally {
      setIsLoading(false)
    }
  }, [search, statusFilter])

  useEffect(() => {
    const t = setTimeout(load, 150)
    return () => clearTimeout(t)
  }, [load])

  const handleDelete = async (id: number, name: string) => {
    if (!confirm(`Delete client "${name}"? This cannot be undone.`)) return
    setDeleteError(null)
    try {
      await invoke('delete_client', { id })
      load()
    } catch (e) {
      setDeleteError(`Could not delete "${name}": ${e}`)
    }
  }

  if (isLoading) {
    return <div className="client-list-loading">Loading clients...</div>
  }

  if (loadError) {
    return (
      <div className="client-list">
        <div className="client-list-loading">
          <p>Failed to load clients</p>
          <Button variant="primary" onClick={load} style={{ marginTop: '8px' }}>
            Retry
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="client-list">
      <div className="list-toolbar">
        <div className="list-search">
          <Input
            placeholder="Search name, company, email..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <Select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
          options={[
            { value: '', label: 'All clients' },
            { value: 'active', label: 'Active' },
            { value: 'inactive', label: 'Inactive' },
          ]}
        />
        <Button variant="primary" onClick={onNewClient}>
          + New Client
        </Button>
      </div>

      {deleteError && <div className="client-delete-error">{deleteError}</div>}

      {clients.length === 0 ? (
        <div className="client-empty">
          <p>{search || statusFilter ? 'No clients match your search.' : 'No clients yet. Add your first client.'}</p>
          {!search && !statusFilter && (
            <Button variant="primary" onClick={onNewClient}>
              Add Client
            </Button>
          )}
        </div>
      ) : (
        <div className="client-table-scroll">
        <table className="client-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Company</th>
              <th>Email</th>
              <th>Phone</th>
              <th>Tags</th>
              <th>Status</th>
              <th></th>
            </tr>
          </thead>
          {clients.length > VIRTUAL_THRESHOLD ? (
            <tbody>
              <tr>
                <td colSpan={7} style={{ padding: 0 }}>
                  <VirtualList
                    items={clients}
                    itemHeight={48}
                    height={520}
                    keyExtractor={(c) => c.id}
                    renderItem={(client) => (
                      <div className="client-row client-row--virtual" onClick={() => onSelectClient(client)}>
                        <span className="col-name">{client.name}</span>
                        <span className="col-company">{client.company || '—'}</span>
                        <span className="col-email">{client.email || '—'}</span>
                        <span className="col-phone">{client.phone || '—'}</span>
                        <span className="col-tags">
                          {client.tags
                            ? client.tags.split(',').map((t) => t.trim()).filter(Boolean).map((tag, i) => (
                                <span key={`${tag}-${i}`} className="tag-chip">{tag}</span>
                              ))
                            : null}
                        </span>
                        <span className="col-status">
                          <Badge tone={statusColors[client.status]}>{client.status}</Badge>
                        </span>
                        <span className="col-actions" onClick={(e) => e.stopPropagation()}>
                          <Button variant="ghost" size="sm" onClick={() => handleDelete(client.id, client.name)}>
                            Delete
                          </Button>
                        </span>
                      </div>
                    )}
                  />
                </td>
              </tr>
            </tbody>
          ) : (
            <tbody>
              {clients.map((client) => (
                <tr key={client.id} className="client-row" onClick={() => onSelectClient(client)}>
                  <td className="col-name">{client.name}</td>
                  <td className="col-company">{client.company || <span className="empty-cell">—</span>}</td>
                  <td className="col-email">
                    {client.email ? (
                      <a href={`mailto:${client.email}`} onClick={(e) => e.stopPropagation()}>
                        {client.email}
                      </a>
                    ) : (
                      <span className="empty-cell">—</span>
                    )}
                  </td>
                  <td className="col-phone">{client.phone || <span className="empty-cell">—</span>}</td>
                  <td className="col-tags">
                    {client.tags
                      ? client.tags
                          .split(',')
                          .map((t) => t.trim())
                          .filter(Boolean)
                          .map((tag, i) => (
                            <span key={`${tag}-${i}`} className="tag-chip">
                              {tag}
                            </span>
                          ))
                      : null}
                  </td>
                  <td className="col-status">
                    <Badge tone={statusColors[client.status]}>{client.status}</Badge>
                  </td>
                  <td className="col-actions" onClick={(e) => e.stopPropagation()}>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(client.id, client.name)}
                    >
                      Delete
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          )}
        </table>
        </div>
      )}

      <div className="list-footer">
        {clients.length} client{clients.length !== 1 ? 's' : ''}
      </div>
    </div>
  )
})
