import { Card, Badge } from '../design-system'
import type { Order } from '../types'
import './OrderListView.css'

interface OrderListViewProps {
  orders: Order[]
}

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

export default function OrderListView({ orders }: OrderListViewProps) {
  const isOverdue = (dueDate: string) => new Date(dueDate) < new Date()

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

      {orders.map((order) => (
        <div key={order.id} className={`list-row ${isOverdue(order.due_date) ? 'overdue' : ''}`}>
          <div className="col-number">
            <span className="order-number">{order.order_number}</span>
          </div>
          <div className="col-description">
            <div className="desc-text">{order.description}</div>
            {order.artwork_approved && <span className="badge-approved">✓ Artwork approved</span>}
          </div>
          <div className="col-status">
            <Badge
              variant={statusColors[order.status]}
              label={statusLabels[order.status] || order.status}
            />
          </div>
          <div className="col-date">
            <span className={isOverdue(order.due_date) ? 'text-danger' : ''}>
              {new Date(order.due_date).toLocaleDateString()}
            </span>
          </div>
          <div className="col-priority">
            <Badge variant={priorityColors[order.priority]} label={order.priority} />
          </div>
          <div className="col-notes">
            {order.deposit_requested && <span>💰 Deposit</span>}
          </div>
        </div>
      ))}
    </div>
  )
}
