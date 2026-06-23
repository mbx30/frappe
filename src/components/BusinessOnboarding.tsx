import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, Select } from '../design-system'
import './BusinessOnboarding.css'

interface BusinessOnboardingProps {
  onComplete: () => void
}

export default function BusinessOnboarding({ onComplete }: BusinessOnboardingProps) {
  const [businessName, setBusinessName] = useState('')
  const [industry, setIndustry] = useState('')
  const [companySize, setCompanySize] = useState('')
  const [orderNumberPrefix, setOrderNumberPrefix] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  const validatePrefix = (value: string): string | null => {
    if (value === '') return null
    if (value.length > 4) return 'Prefix must be at most 4 characters'
    if (!/^[A-Za-z0-9]+$/.test(value)) return 'Prefix must be alphanumeric only (A-Z, 0-9)'
    return null
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!businessName || !industry || !companySize) {
      setError('Please fill in all fields')
      return
    }
    const prefixError = validatePrefix(orderNumberPrefix)
    if (prefixError) {
      setError(prefixError)
      return
    }

    setIsLoading(true)
    setError('')
    try {
      await invoke('save_business_info', {
        businessName,
        industry,
        companySize,
        orderNumberPrefix,
      })
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
            <Input
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
            <Select
              id="industry"
              value={industry}
              onChange={(e) => setIndustry(e.target.value)}
              disabled={isLoading}
              options={[
                { value: '', label: 'Select an industry' },
                { value: 'commercial', label: 'Commercial Printing' },
                { value: 'packaging', label: 'Packaging' },
                { value: 'promotional', label: 'Promotional Products' },
                { value: 'signage', label: 'Signage & Graphics' },
                { value: 'textile', label: 'Textile Printing' },
                { value: 'direct_mail', label: 'Direct Mail' },
                { value: 'wide_format', label: 'Wide Format' },
                { value: 'other', label: 'Other' },
              ]}
            />
          </div>

          <div className="form-group">
            <label htmlFor="companySize">Company Size</label>
            <Select
              id="companySize"
              value={companySize}
              onChange={(e) => setCompanySize(e.target.value)}
              disabled={isLoading}
              options={[
                { value: '', label: 'Select a size' },
                { value: '1', label: 'Just me (1 person)' },
                { value: '2-5', label: 'Small (2-5 people)' },
                { value: '6-20', label: 'Medium (6-20 people)' },
                { value: '20+', label: 'Large (20+ people)' },
              ]}
            />
          </div>

          <div className="form-group">
            <label htmlFor="orderNumberPrefix">
              Order Number Prefix <span className="hint">(optional, max 4 alphanumeric)</span>
            </label>
            <Input
              id="orderNumberPrefix"
              type="text"
              value={orderNumberPrefix}
              onChange={(e) => {
                // `maxLength` only caps typed input; pasted content can bypass it.
                // Truncate to 4 chars after uppercasing so visual state matches
                // the validator's length check.
                const v = e.target.value.toUpperCase().slice(0, 4)
                setOrderNumberPrefix(v)
                const err = validatePrefix(v)
                setError(err ?? '')
              }}
              placeholder="e.g., ORD, JOB, INV"
              disabled={isLoading}
              maxLength={4}
              pattern="[A-Za-z0-9]{0,4}"
            />
            <p className="hint">
              Leave blank for plain incrementing numbers (e.g., 0001, 0002). Add a prefix to
              match your existing scheme (e.g., ORD-0001, JOB-0001).
            </p>
          </div>

          {error && <div className="error-message">{error}</div>}

          <div className="form-actions">
            <Button
              variant="primary"
              fullWidth
              type="submit"
              disabled={isLoading}
            >
              {isLoading ? 'Saving...' : 'Get Started'}
            </Button>
            <Button
              variant="secondary"
              fullWidth
              type="button"
              onClick={handleSkip}
              disabled={isLoading}
            >
              Skip for now
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}
