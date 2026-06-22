import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { BusinessInfo } from './types'
import Welcome from './components/Welcome'
import BusinessOnboarding from './components/BusinessOnboarding'
import ManagementView from './components/ManagementView'
import './App.css'

function App() {
  const [businessInfo, setBusinessInfo] = useState<BusinessInfo | null>(null)
  const [onboardingStep, setOnboardingStep] = useState<'welcome' | 'business' | 'complete' | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  const checkOnboarding = useCallback(async () => {
    try {
      const info = await invoke<BusinessInfo | null>('get_business_info')
      setBusinessInfo(info)
      if (!info?.completed_onboarding) {
        setOnboardingStep('welcome')
      } else {
        setOnboardingStep('complete')
      }
    } catch (e) {
      console.error('Failed to check onboarding:', e)
      setOnboardingStep('welcome')
    } finally {
      setIsLoading(false)
    }
  }, [])

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { checkOnboarding() }, [checkOnboarding])

  if (isLoading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>
        Loading...
      </div>
    )
  }

  if (onboardingStep === 'welcome') {
    return <Welcome onImportComplete={() => setOnboardingStep('business')} />
  }

  if (onboardingStep === 'business') {
    return (
      <BusinessOnboarding
        onComplete={() => {
          setOnboardingStep('complete')
          checkOnboarding()
        }}
      />
    )
  }

  return (
    <div className="app-shell">
      <div className="app-tab-bar">
        <div className="app-brand">Frappe</div>
        <div className="app-tabs">
          <button
            className={`app-tab app-tab--active`}
          >
            Management
          </button>
        </div>
        <div className="app-tab-bar-end">
          {businessInfo?.business_name && (
            <span className="biz-name">{businessInfo.business_name}</span>
          )}
        </div>
      </div>

      <div className="app-content">
        <ManagementView />
      </div>
    </div>
  )
}

export default App
