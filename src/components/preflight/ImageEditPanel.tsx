import { memo } from 'react'

interface ImageEditPanelProps {
  jobId?: number
}

export default memo(function ImageEditPanel({ jobId }: ImageEditPanelProps) {
  return (
    <div className="image-edit-panel">
      <h3>Image Editor</h3>
      <p>Click a rendered image to inspect, replace, or optimize it. (Scaffold for issue #263)</p>
      {jobId && <p>Job: {jobId}</p>}
    </div>
  )
})
