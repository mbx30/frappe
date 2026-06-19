import { useState, useEffect } from 'react'
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
  const [filteredOrders, setFilteredOrders] = useState<Order[]>([])
  const [viewMode, setViewMode] = useState<ViewMode>('kanban')
  const [isLoading, setIsLoading] = useState(true)
  const [stats, setStats] = useState<DashboardStats>({
    total: 0,
    prepress: 0,
    production: 0,
    delivery: 0,
    completed: 0,
    overdue: 0,
    dueToday: 0,
  })
  const [searchText, setSearchText] = useState('')
  const [filterStatus, setFilterStatus] = useState<string | null>(null)
  const [filterPriority, setFilterPriority] = useState<string | null>(null)

  useEffect(() => {
    loadOrders()
  }, [])

  useEffect(() => {
    applyFilters()
  }, [orders, searchText, filterStatus, filterPriority])

  const loadOrders = async () => {
    try {
      const result = await invoke<Order[]>('list_orders')
      const activeOrders = result.filter((o) => o.status !== 'completed')
      setOrders(activeOrders)
      calculateStats(activeOrders)
    } catch (e) {
      console.error('Failed to load orders:', e)
    } finally {
      setIsLoading(false)
    }
  }

  const calculateStats = (orderList: Order[]) => {
    const today = new Date().toISOString().split('T')[0]
    let overdue = 0
    let dueToday = 0

    orderList.forEach((o) => {
      if (o.due_date < today) overdue++
      if (o.due_date === today) dueToday++
    })

    setStats({
      total: orderList.length,
      prepress: orderList.filter((o) => o.status === 'prepress').length,
      production: orderList.filter((o) => o.status === 'production').length,
      delivery: orderList.filter((o) => o.status === 'delivery').length,
      completed: orderList.filter((o) => o.status === 'completed').length,
      overdue,
      dueToday,
    })
  }

  const applyFilters = () => {
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

    setFilteredOrders(filtered)
  }

  if (isLoading) {
    return (
      <div className="dashboard-container">
        <div className="loading">Loading dashboard...</div>
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
