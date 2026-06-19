import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Card, Badge } from '../design-system'
import type { Order } from '../types'
import './OrderList.css'

interface OrderListProps {
  onCreateNew: () => void
  onSelectOrder: (id: number) => void
}

const priorityColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  low: 'info',
  normal: 'info',
  high: 'warning',
  urgent: 'danger',
}

const statusColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  prepress: 'info',
  production: 'warning',
  delivery: 'warning',
  completed: 'success',
}

const statusLabels: Record<string, string> = {
  prepress: 'Pre-press',
  production: 'Production',
  delivery: 'Delivery',
  completed: 'Completed',
}

export default function OrderList({ onCreateNew, onSelectOrder }: OrderListProps) {
  const [orders, setOrders] = useState<Order[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [filterStatus, setFilterStatus] = useState<string | null>(null)

  useEffect(() => {
    loadOrders()
  }, [])

  const loadOrders = async () => {
    try {
      const result = await invoke<Order[]>('list_orders')
      setOrders(result)
    } catch (e) {
      console.error('Failed to load orders:', e)
    } finally {
      setIsLoading(false)
    }
  }

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString()
  }

  const filteredOrders = filterStatus
    ? orders.filter((o) => o.status === filterStatus)
    : orders

  const ordersByStatus = {
    prepress: orders.filter((o) => o.status === 'prepress'),
    production: orders.filter((o) => o.status === 'production'),
    delivery: orders.filter((o) => o.status === 'delivery'),
    completed: orders.filter((o) => o.status === 'completed'),
  }

  if (isLoading) {
    return (
      <div className="order-list-container">
        <div className="loading">Loading orders...</div>
      </div>
    )
  }

  return (
    <div className="order-list-container">
      <div className="order-header">
        <div>
          <h2>Orders</h2>
          <p className="subtitle">{orders.length} total</p>
        </div>
        <Button variant="primary" onClick={onCreateNew}>
          + New Order
        </Button>
      </div>

      {/* Kanban view */}
      <div className="kanban-board">
        {(['prepress', 'production', 'delivery', 'completed'] as const).map((status) => (
          <div key={status} className="kanban-column">
            <div className="column-header">
              <h3>{statusLabels[status]}</h3>
              <span className="column-count">{ordersByStatus[status].length}</span>
            </div>
            <div className="column-cards">
              {ordersByStatus[status].length === 0 ? (
                <div className="empty-column">No orders</div>
              ) : (
                ordersByStatus[status].map((order) => (
                  <Card
                    key={order.id}
                    className="order-card"
                    onClick={() => onSelectOrder(order.id)}
                  >
                    <div className="card-header">
                      <div>
                        <div className="order-number">{order.order_number}</div>
                        <p className="order-description">{order.description}</p>
                      </div>
                      <Badge
                        variant={priorityColors[order.priority]}
                        label={order.priority}
                      />
                    </div>

                    <div className="card-body">
                      <div className="card-row">
                        <span className="label">Due:</span>
                        <span>{formatDate(order.due_date)}</span>
                      </div>
                      {order.artwork_approved && (
                        <div className="card-row">
                          <span className="label">✓ Artwork approved</span>
                        </div>
                      )}
                      {order.deposit_requested && (
                        <div className="card-row">
                          <span className="label">Deposit requested</span>
                        </div>
                      )}
                    </div>
                  </Card>
                ))
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
