import { memo } from 'react'
import { t } from '../../i18n'

interface HelpPanelProps {
  article?: string
}

export default memo(function HelpPanel({ article }: HelpPanelProps) {
  return (
    <div className="help-panel">
      <h3>{t('help.title')}</h3>
      <p>{t('help.desc')}</p>
      {article && <p>{t('help.article', { name: article })}</p>}
    </div>
  )
})
