import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Card, Badge } from '../design-system'
import type { Order } from '../types'
import './OrderKanban.css'

interface OrderKanbanProps {
  orders: Order[]
  onOrdersChange: () => void
}

const statusLabels: Record<string, string> = {
  prepress: 'Pre-press',
  production: 'Production',
  delivery: 'Delivery',
  completed: 'Completed',
}

const priorityColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  low: 'info',
  normal: 'info',
  high: 'warning',
  urgent: 'danger',
}

export default function OrderKanban({ orders, onOrdersChange }: OrderKanbanProps) {
  const [draggedOrder, setDraggedOrder] = useState<Order | null>(null)

  const statusOrder = ['prepress', 'production', 'delivery', 'completed'] as const

  const ordersByStatus = statusOrder.reduce(
    (acc, status) => {
      acc[status] = orders.filter((o) => o.status === status)
      return acc
    },
    {} as Record<string, Order[]>
  )

  const isOverdue = (dueDate: string) => {
    return new Date(dueDate) < new Date() && new Date(dueDate).toDateString() !== new Date().toDateString()
  }

  const isDueToday = (dueDate: string) => {
    return new Date(dueDate).toDateString() === new Date().toDateString()
  }

  const handleDragStart = (order: Order) => {
    setDraggedOrder(order)
  }

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
  }

  const handleDrop = async (status: string) => {
    if (!draggedOrder || draggedOrder.status === status) {
      setDraggedOrder(null)
      return
    }

    try {
      await invoke('update_order_status', {
        order_id: draggedOrder.id,
        new_status: status,
        notes: `Moved to ${statusLabels[status]} from kanban board`,
      })
      onOrdersChange()
    } catch (e) {
      console.error('Failed to update order status:', e)
    } finally {
      setDraggedOrder(null)
    }
  }

  return (
    <div className="kanban-board">
      {statusOrder.map((status) => (
        <div key={status} className="kanban-column">
          <div className="column-header">
            <h3>{statusLabels[status]}</h3>
            <span className="column-count">{ordersByStatus[status].length}</span>
          </div>

          <div
            className="column-cards"
            onDragOver={handleDragOver}
            onDrop={() => handleDrop(status)}
          >
            {ordersByStatus[status].length === 0 ? (
              <div className="empty-column">No orders</div>
            ) : (
              ordersByStatus[status].map((order) => (
                <div
                  key={order.id}
                  className={`order-card ${draggedOrder?.id === order.id ? 'dragging' : ''}`}
                  draggable
                  onDragStart={() => handleDragStart(order)}
                >
                  <div className="card-header">
                    <div className="order-number">{order.order_number}</div>
                    <Badge
                      variant={priorityColors[order.priority]}
                      label={order.priority}
                    />
                  </div>

                  <div className="card-body">
                    <div className="description">{order.description}</div>

                    <div className="order-meta">
                      <span
                        className={`due-date ${isOverdue(order.due_date) ? 'overdue' : isDueToday(order.due_date) ? 'due-today' : ''}`}
                      >
                        {new Date(order.due_date).toLocaleDateString(undefined, {
                          month: 'short',
                          day: 'numeric',
                        })}
                      </span>

                      {order.artwork_approved && (
                        <span className="badge-approved">✓ Art</span>
                      )}

                      {order.deposit_requested && (
                        <span className="badge-deposit">$ Deposit</span>
                      )}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      ))}
    </div>
  )
}
