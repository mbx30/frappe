import { memo } from 'react'

interface AnalyticsDashboardProps {
  clientId?: number
}

export default memo(function AnalyticsDashboard({ clientId }: AnalyticsDashboardProps) {
  return (
    <div className="analytics-dashboard">
      <h3>Preflight Analytics</h3>
      <p>Pass rate, top errors, and per-client views. (Scaffold for issue #271)</p>
      {clientId && <p>Client: {clientId}</p>}
    </div>
  )
})
