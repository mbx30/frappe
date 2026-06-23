import type { Workbook } from '../types'
import { Button } from '../design-system'

interface WorkbookListProps {
  workbooks: Workbook[]
  activeId: number | null
  onSelect: (id: number) => void
  onCreate: () => void
  onDelete: (id: number) => void
}

export default function WorkbookList({ workbooks, activeId, onSelect, onCreate, onDelete }: WorkbookListProps) {
  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <h3>Frappe</h3>
        <Button variant="primary" size="sm" onClick={onCreate}>
          + New Workbook
        </Button>
      </div>
      <div className="sidebar-list">
        {workbooks.length === 0 && (
          <p className="sidebar-empty">No workbooks yet. Create one to get started.</p>
        )}
        {workbooks.map((wb) => (
          <div
            key={wb.id}
            className={`sidebar-item ${wb.id === activeId ? 'active' : ''}`}
            onClick={() => onSelect(wb.id)}
          >
            <span className="sidebar-item-name">{wb.name}</span>
            <button
              className="sidebar-item-delete"
              onClick={(e) => { e.stopPropagation(); onDelete(wb.id) }}
              aria-label="Delete workbook"
              title="Delete"
            >
              ×
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}
