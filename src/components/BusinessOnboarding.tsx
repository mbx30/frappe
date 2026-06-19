import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './BusinessOnboarding.css'

interface BusinessOnboardingProps {
  onComplete: () => void
}

export default function BusinessOnboarding({ onComplete }: BusinessOnboardingProps) {
  const [businessName, setBusinessName] = useState('')
  const [industry, setIndustry] = useState('')
  const [companySize, setCompanySize] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!businessName || !industry || !companySize) {
      setError('Please fill in all fields')
      return
    }

    setIsLoading(true)
    setError('')
    try {
      await invoke('save_business_info', { businessName, industry, companySize })
      onComplete()
    } catch (e) {
      setError(`Failed to save business info: ${e}`)
    } finally {
      setIsLoading(false)
    }
  }

  const handleSkip = () => {
    onComplete()
  }

  return (
    <div className="onboarding-container">
      <div className="onboarding-content">
        <h1>Tell us about your business</h1>
        <p>This helps us personalize your Frappe experience</p>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="businessName">Business Name</label>
            <input
              id="businessName"
              type="text"
              value={businessName}
              onChange={(e) => setBusinessName(e.target.value)}
              placeholder="e.g., Acme Print Co."
              disabled={isLoading}
            />
          </div>

          <div className="form-group">
            <label htmlFor="industry">Industry</label>
            <select
              id="industry"
              value={industry}
              onChange={(e) => setIndustry(e.target.value)}
              disabled={isLoading}
            >
              <option value="">Select an industry</option>
              <option value="commercial">Commercial Printing</option>
              <option value="packaging">Packaging</option>
              <option value="promotional">Promotional Products</option>
              <option value="signage">Signage & Graphics</option>
              <option value="textile">Textile Printing</option>
              <option value="direct_mail">Direct Mail</option>
              <option value="wide_format">Wide Format</option>
              <option value="other">Other</option>
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="companySize">Company Size</label>
            <select
              id="companySize"
              value={companySize}
              onChange={(e) => setCompanySize(e.target.value)}
              disabled={isLoading}
            >
              <option value="">Select a size</option>
              <option value="1">Just me (1 person)</option>
              <option value="2-5">Small (2-5 people)</option>
              <option value="6-20">Medium (6-20 people)</option>
              <option value="20+">Large (20+ people)</option>
            </select>
          </div>

          {error && <div className="error-message">{error}</div>}

          <div className="form-actions">
            <button
              type="submit"
              className="btn btn-primary"
              disabled={isLoading}
            >
              {isLoading ? 'Saving...' : 'Get Started'}
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={handleSkip}
              disabled={isLoading}
            >
              Skip for now
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
