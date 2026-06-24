import { memo } from 'react'
import { Card, Badge } from '../design-system'
import type { Order } from '../types'
import { VirtualList } from './common/VirtualList'
import './OrderListView.css'

interface OrderListViewProps {
  orders: Order[]
}

const VIRTUAL_THRESHOLD = 200

const statusColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  prepress: 'info',
  production: 'warning',
  delivery: 'warning',
  completed: 'success',
}

const priorityColors: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  low: 'info',
  normal: 'info',
  high: 'warning',
  urgent: 'danger',
}

const statusLabels: Record<string, string> = {
  prepress: 'Pre-press',
  production: 'Production',
  delivery: 'Delivery',
  completed: 'Completed',
}

const OrderRow = memo(function OrderRow({ order, isOverdue }: { order: Order; isOverdue: boolean }) {
  return (
    <div className={`list-row ${isOverdue ? 'overdue' : ''}`}>
      <div className="col-number">
        <span className="order-number">{order.order_number}</span>
      </div>
      <div className="col-description">
        <div className="desc-text">{order.description}</div>
        {order.artwork_approved && <span className="badge-approved">✓ Artwork approved</span>}
      </div>
      <div className="col-status">
        <Badge
          tone={statusColors[order.status]}
        >
          {statusLabels[order.status] || order.status}
        </Badge>
      </div>
      <div className="col-date">
        <span className={isOverdue ? 'text-danger' : ''}>
          {new Date(order.due_date).toLocaleDateString()}
        </span>
      </div>
      <div className="col-priority">
        <Badge tone={priorityColors[order.priority]}>{order.priority}</Badge>
      </div>
      <div className="col-notes">
        {order.deposit_requested && <span>💰 Deposit</span>}
      </div>
    </div>
  )
})

function OrderListView({ orders }: OrderListViewProps) {
  const todayStr = new Date().toISOString().split('T')[0]

  if (orders.length === 0) {
    return (
      <div className="order-list">
        <Card className="empty-state">
          <div className="empty-content">
            <h3>No orders to show</h3>
            <p>Try adjusting your filters</p>
          </div>
        </Card>
      </div>
    )
  }

  const useVirtual = orders.length > VIRTUAL_THRESHOLD

  return (
    <div className="order-list">
      <div className="list-header">
        <div className="col-number">Order #</div>
        <div className="col-description">Description</div>
        <div className="col-status">Status</div>
        <div className="col-date">Due Date</div>
        <div className="col-priority">Priority</div>
        <div className="col-notes">Notes</div>
      </div>

      {useVirtual ? (
        <VirtualList
          items={orders}
          itemHeight={56}
          height={520}
          keyExtractor={(o) => o.id}
          renderItem={(order) => (
            <OrderRow order={order} isOverdue={order.due_date < todayStr} />
          )}
          className="virtual-order-list"
        />
      ) : (
        orders.map((order) => (
          <OrderRow
            key={order.id}
            order={order}
            isOverdue={order.due_date < todayStr}
          />
        ))
      )}
    </div>
  )
}

export default memo(OrderListView)
