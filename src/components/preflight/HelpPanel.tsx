import { memo } from 'react'

interface HelpPanelProps {
  article?: string
}

export default memo(function HelpPanel({ article }: HelpPanelProps) {
  return (
    <div className="help-panel">
      <h3>Help</h3>
      <p>Searchable help articles for PDF tooling. (Scaffold for issue #276)</p>
      {article && <p>Article: {article}</p>}
    </div>
  )
})
