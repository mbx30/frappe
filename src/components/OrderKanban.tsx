import { useState, useMemo, useCallback, memo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Badge } from '../design-system'
import type { Order } from '../types'
import './OrderKanban.css'

interface OrderKanbanProps {
  orders: Order[]
  onOrdersChange: () => void
}

interface OrderCardProps {
  order: Order
  isDragging: boolean
  isOverdue: boolean
  isDueToday: boolean
  onDragStart: (e: React.DragEvent, order: Order) => void
}

const OrderCard = memo(function OrderCard({
  order,
  isDragging,
  isOverdue,
  isDueToday,
  onDragStart,
}: OrderCardProps) {
  return (
    <div
      className={`order-card ${isDragging ? 'dragging' : ''}`}
      draggable
      onDragStart={(e) => onDragStart(e, order)}
    >
      <div className="card-header">
        <div className="order-number">{order.order_number}</div>
        <Badge tone={priorityColors[order.priority]}>{order.priority}</Badge>
      </div>

      <div className="card-body">
        <div className="description">{order.description}</div>

        <div className="order-meta">
          <span
            className={`due-date ${
              isOverdue ? 'overdue' : isDueToday ? 'due-today' : ''
            }`}
          >
            {(() => {
              const [y, m, d] = order.due_date.split('-').map(Number)
              return new Date(y, m - 1, d).toLocaleDateString(undefined, {
                month: 'short',
                day: 'numeric',
              })
            })()}
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
  )
})

const statusLabels: Record<string, string> = {
  prepress: 'Pre-press',
  production: 'Production',
  delivery: 'Delivery',
  completed: 'Completed',
}

const statusOrder = Object.keys(statusLabels)

const priorityColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  low: 'info',
  normal: 'info',
  high: 'warning',
  urgent: 'danger',
}

const VALID_TRANSITIONS: Record<string, string[]> = {
  prepress: ['production'],
  production: ['delivery'],
  delivery: ['completed'],
  completed: [],
}

export default memo(function OrderKanban({ orders, onOrdersChange }: OrderKanbanProps) {
  const [draggedOrder, setDraggedOrder] = useState<Order | null>(null)

  const ordersByStatus = useMemo(() => {
    const acc: Record<string, Order[]> = {
      prepress: [],
      production: [],
      delivery: [],
      completed: [],
    }
    for (const o of orders) {
      if (acc[o.status]) acc[o.status].push(o)
    }
    return acc
  }, [orders])

  const todayStr = useMemo(() => new Date().toISOString().split('T')[0], [])

  const isOverdue = useCallback(
    (dueDate: string) => dueDate < todayStr,
    [todayStr]
  )

  const isDueToday = useCallback(
    (dueDate: string) => dueDate === todayStr,
    [todayStr]
  )

  const handleDragStart = useCallback((e: React.DragEvent, order: Order) => {
    e.dataTransfer.setData('text/plain', order.id.toString())
    e.dataTransfer.effectAllowed = 'move'
    setDraggedOrder(order)
  }, [])

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
  }, [])

  const handleDrop = useCallback(
    async (status: string) => {
      if (!draggedOrder || draggedOrder.status === status) {
        setDraggedOrder(null)
        return
      }

      // Validate transition — prevent backward moves on the kanban board
      if (!VALID_TRANSITIONS[draggedOrder.status]?.includes(status)) {
        setDraggedOrder(null)
        return
      }

      try {
        await invoke('update_order_status', {
          orderId: draggedOrder.id,
          newStatus: status,
          notes: `Moved to ${statusLabels[status]} from kanban board`,
        })
        onOrdersChange()
      } catch (e) {
        console.error('Failed to update order status:', e)
      } finally {
        setDraggedOrder(null)
      }
    },
    [draggedOrder, onOrdersChange]
  )

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
                <OrderCard
                  key={order.id}
                  order={order}
                  isDragging={draggedOrder?.id === order.id}
                  isOverdue={isOverdue(order.due_date)}
                  isDueToday={isDueToday(order.due_date)}
                  onDragStart={handleDragStart}
                />
              ))
            )}
          </div>
        </div>
      ))}
    </div>
  )
})
