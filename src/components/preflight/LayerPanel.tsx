import { memo } from 'react'

interface LayerPanelProps {
  jobId?: number
}

export default memo(function LayerPanel({ jobId }: LayerPanelProps) {
  return (
    <div className="layer-panel">
      <h3>Layers</h3>
      <p>Toggle optional content group visibility. (Scaffold for issue #264)</p>
      {jobId && <p>Job: {jobId}</p>}
    </div>
  )
})
