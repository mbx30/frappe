import { memo } from 'react'
import { t } from '../../i18n'

interface BarcodeCheckProps {
  jobId?: number
}

export default memo(function BarcodeCheck({ jobId }: BarcodeCheckProps) {
  return (
    <div className="barcode-check">
      <h3>{t('barcode.check.title')}</h3>
      <p>{t('barcode.check.desc')}</p>
      {jobId && <p>{t('barcode.check.job', { id: jobId })}</p>}
    </div>
  )
})
