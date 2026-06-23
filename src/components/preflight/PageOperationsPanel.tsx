import { memo } from 'react'

interface PageOperationsPanelProps {
  jobId?: number
}

export default memo(function PageOperationsPanel({ jobId }: PageOperationsPanelProps) {
  return (
    <div className="page-operations-panel">
      <h3>Page Operations</h3>
      <p>Extract, delete, rotate, reorder, or insert blank pages. (Scaffold for issue #264)</p>
      {jobId && <p>Job: {jobId}</p>}
    </div>
  )
})
