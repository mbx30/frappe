import { memo } from 'react'

interface BarcodeCheckProps {
  jobId?: number
}

export default memo(function BarcodeCheck({ jobId }: BarcodeCheckProps) {
  return (
    <div className="barcode-check">
      <h3>Barcode Check</h3>
      <p>Detect and validate barcodes with quiet-zone and size checks. (Scaffold for issue #270)</p>
      {jobId && <p>Job: {jobId}</p>}
    </div>
  )
})
