import { Input, Select, Button } from '../design-system'
import './DashboardFilters.css'

interface DashboardFiltersProps {
  searchText: string
  onSearchChange: (text: string) => void
  filterStatus: string | null
  onStatusChange: (status: string | null) => void
  filterPriority: string | null
  onPriorityChange: (priority: string | null) => void
}

export default function DashboardFilters({
  searchText,
  onSearchChange,
  filterStatus,
  onStatusChange,
  filterPriority,
  onPriorityChange,
}: DashboardFiltersProps) {
  const handleClearFilters = () => {
    onSearchChange('')
    onStatusChange(null)
    onPriorityChange(null)
  }

  const hasFilters = searchText || filterStatus || filterPriority

  return (
    <div className="filters-bar">
      <Input
        placeholder="Search by order # or description..."
        value={searchText}
        onChange={(e) => onSearchChange(e.target.value)}
        type="text"
      />

      <Select
        value={filterStatus || ''}
        onChange={(e) => onStatusChange(e.target.value || null)}
        options={[
          { value: '', label: 'All Status' },
          { value: 'prepress', label: 'Pre-press' },
          { value: 'production', label: 'Production' },
          { value: 'delivery', label: 'Delivery' },
          { value: 'completed', label: 'Completed' },
        ]}
      />

      <Select
        value={filterPriority || ''}
        onChange={(e) => onPriorityChange(e.target.value || null)}
        options={[
          { value: '', label: 'All Priority' },
          { value: 'low', label: 'Low' },
          { value: 'normal', label: 'Normal' },
          { value: 'high', label: 'High' },
          { value: 'urgent', label: 'Urgent' },
        ]}
      />

      {hasFilters && (
        <Button variant="secondary" size="sm" onClick={handleClearFilters}>
          Clear Filters
        </Button>
      )}
    </div>
  )
}
