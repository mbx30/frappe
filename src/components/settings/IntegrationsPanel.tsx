import { memo } from 'react'

export default memo(function IntegrationsPanel() {
  return (
    <div className="integrations-panel">
      <h3>Integrations</h3>
      <p>SMTP, FTP, and MIS webhook settings. (Scaffold for issue #274)</p>
    </div>
  )
})
