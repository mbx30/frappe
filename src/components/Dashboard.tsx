import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card } from '../design-system'
import type { Order } from '../types'
import OrderKanban from './OrderKanban'
import OrderListView from './OrderListView'
import DashboardFilters from './DashboardFilters'
import './Dashboard.css'

type ViewMode = 'list' | 'kanban' | 'calendar'

interface DashboardStats {
  total: number
  prepress: number
  production: number
  delivery: number
  completed: number
  overdue: number
  dueToday: number
}

export default function Dashboard() {
  const [orders, setOrders] = useState<Order[]>([])
  const [viewMode, setViewMode] = useState<ViewMode>('kanban')
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [searchText, setSearchText] = useState('')
  const [filterStatus, setFilterStatus] = useState<string | null>(null)
  const [filterPriority, setFilterPriority] = useState<string | null>(null)

  const stats = useMemo<DashboardStats>(() => {
    const today = new Date().toISOString().split('T')[0]
    let overdue = 0
    let dueToday = 0
    orders.filter((o) => o.status !== 'completed').forEach((o) => {
      if (o.due_date < today) overdue++
      if (o.due_date === today) dueToday++
    })
    return {
      total: orders.length,
      prepress: orders.filter((o) => o.status === 'prepress').length,
      production: orders.filter((o) => o.status === 'production').length,
      delivery: orders.filter((o) => o.status === 'delivery').length,
      completed: orders.filter((o) => o.status === 'completed').length,
      overdue,
      dueToday,
    }
  }, [orders])

  const filteredOrders = useMemo(() => {
    let filtered = [...orders]
    if (searchText) {
      const search = searchText.toLowerCase()
      filtered = filtered.filter(
        (o) =>
          o.order_number.toLowerCase().includes(search) ||
          o.description.toLowerCase().includes(search)
      )
    }
    if (filterStatus) {
      filtered = filtered.filter((o) => o.status === filterStatus)
    }
    if (filterPriority) {
      filtered = filtered.filter((o) => o.priority === filterPriority)
    }
    return filtered
  }, [orders, searchText, filterStatus, filterPriority])

  const loadOrders = useCallback(async () => {
    setIsLoading(true)
    setLoadError(null)
    try {
      const result = await invoke<Order[]>('list_orders')
      setOrders(result)
    } catch (e) {
      console.error('Failed to load orders:', e)
      setLoadError(String(e))
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadOrders()
  }, [loadOrders])

  if (isLoading) {
    return (
      <div className="dashboard-container">
        <div className="loading">Loading dashboard...</div>
      </div>
    )
  }

  if (loadError) {
    return (
      <div className="dashboard-container">
        <div className="dashboard-header">
          <h1>Dashboard</h1>
        </div>
        <Card className="empty-state">
          <div className="empty-content">
            <h3>Failed to load dashboard</h3>
            <p>{loadError}</p>
            <Button variant="primary" onClick={loadOrders} style={{ marginTop: '16px' }}>
              Retry
            </Button>
          </div>
        </Card>
      </div>
    )
  }

  return (
    <div className="dashboard-container">
      {/* Header */}
      <div className="dashboard-header">
        <div>
          <h1>Dashboard</h1>
          <p className="subtitle">
            {filteredOrders.length} active order
            {filteredOrders.length !== 1 ? 's' : ''}
          </p>
        </div>

        {/* View switcher */}
        <div className="view-switcher">
          <Button
            variant={viewMode === 'list' ? 'primary' : 'secondary'}
            size="sm"
            onClick={() => setViewMode('list')}
          >
            List
          </Button>
          <Button
            variant={viewMode === 'kanban' ? 'primary' : 'secondary'}
            size="sm"
            onClick={() => setViewMode('kanban')}
          >
            Kanban
          </Button>
          <Button
            variant={viewMode === 'calendar' ? 'primary' : 'secondary'}
            size="sm"
            onClick={() => setViewMode('calendar')}
            disabled
          >
            Calendar (Coming soon)
          </Button>
        </div>
      </div>

      {/* Stats */}
      <div className="stats-row">
        <Card className="stat-card">
          <div className="stat-value">{stats.total}</div>
          <div className="stat-label">Total Active</div>
        </Card>
        <Card className="stat-card">
          <div className="stat-value">{stats.prepress}</div>
          <div className="stat-label">Pre-press</div>
        </Card>
        <Card className="stat-card">
          <div className="stat-value">{stats.production}</div>
          <div className="stat-label">Production</div>
        </Card>
        <Card className="stat-card">
          <div className="stat-value">{stats.delivery}</div>
          <div className="stat-label">Delivery</div>
        </Card>
        <Card className="stat-card priority-stat">
          <div className="stat-value">{stats.overdue}</div>
          <div className="stat-label">Overdue</div>
        </Card>
        <Card className="stat-card">
          <div className="stat-value">{stats.dueToday}</div>
          <div className="stat-label">Due Today</div>
        </Card>
      </div>

      {/* Filters */}
      <DashboardFilters
        searchText={searchText}
        onSearchChange={setSearchText}
        filterStatus={filterStatus}
        onStatusChange={setFilterStatus}
        filterPriority={filterPriority}
        onPriorityChange={setFilterPriority}
      />

      {/* Content */}
      <div className="dashboard-content">
        {filteredOrders.length === 0 ? (
          <Card className="empty-state">
            <div className="empty-content">
              <h3>No orders found</h3>
              <p>Try adjusting your filters</p>
            </div>
          </Card>
        ) : viewMode === 'list' ? (
          <OrderListView orders={filteredOrders} />
        ) : (
          <OrderKanban orders={filteredOrders} onOrdersChange={loadOrders} />
        )}
      </div>
    </div>
  )
}
