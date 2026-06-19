/**
 * Example component demonstrating Frappe Design System usage
 * Remove this file after you've reviewed it and understand the patterns
 */

import { useState } from 'react'
import {
  Button,
  Input,
  Select,
  Badge,
  Card,
  Dialog,
  Tabs,
  IconButton,
  Checkbox
} from '../design-system'
import { ChevronDown, Plus, Trash2 } from 'lucide-react'

export function DesignSystemExample() {
  const [isOpen, setIsOpen] = useState(false)
  const [activeTab, setActiveTab] = useState('overview')

  return (
    <div style={{ padding: 'var(--space-16)', maxWidth: '800px', margin: '0 auto' }}>
      {/* Typography & Spacing Example */}
      <h1 style={{ font: 'var(--font-h1)', marginBottom: 'var(--space-8)' }}>
        Frappe Design System Examples
      </h1>

      <p style={{ font: 'var(--font-body)', color: 'var(--text-secondary)', marginBottom: 'var(--space-12)' }}>
        Below are examples of the design system components and tokens in action.
      </p>

      {/* Tabs Navigation */}
      <Tabs
        activeTab={activeTab}
        onChange={setActiveTab}
        tabs={[
          { id: 'overview', label: 'Overview' },
          { id: 'forms', label: 'Forms' },
          { id: 'display', label: 'Display' }
        ]}
        style={{ marginBottom: 'var(--space-16)' }}
      />

      {/* Card Container */}
      <Card style={{ marginBottom: 'var(--space-16)' }}>
        {activeTab === 'overview' && (
          <div style={{ padding: 'var(--space-12)' }}>
            <h2 style={{ font: 'var(--font-h2)', marginBottom: 'var(--space-6)' }}>
              Welcome to Frappe
            </h2>
            <p style={{ font: 'var(--font-body)', lineHeight: 'var(--leading-normal)', marginBottom: 'var(--space-8)' }}>
              This is a local-first print shop MIS built for speed and utility. The design system ensures consistency across all interfaces.
            </p>
            <div style={{ display: 'flex', gap: 'var(--space-4)', flexWrap: 'wrap' }}>
              <Badge tone="success">Production ready</Badge>
              <Badge tone="info">Open source</Badge>
              <Badge tone="warning">In development</Badge>
            </div>
          </div>
        )}

        {activeTab === 'forms' && (
          <div style={{ padding: 'var(--space-12)', display: 'grid', gap: 'var(--space-8)' }}>
            <Input
              label="Order ID"
              placeholder="e.g., INV-1234"
              type="text"
            />

            <Select
              label="Status"
              options={[
                { value: 'draft', label: 'Draft' },
                { value: 'queued', label: 'Queued' },
                { value: 'approved', label: 'Approved' },
                { value: 'on-press', label: 'On press' }
              ]}
            />

            <div>
              <label style={{ font: 'var(--font-label)', color: 'var(--text-primary)', display: 'block', marginBottom: 'var(--space-3)' }}>
                Options
              </label>
              <Checkbox label="Rush shipping" />
              <Checkbox label="Custom handling" style={{ marginTop: 'var(--space-3)' }} />
            </div>

            <div style={{ display: 'flex', gap: 'var(--space-4)' }}>
              <Button
                variant="primary"
                onClick={() => setIsOpen(true)}
              >
                Submit order
              </Button>
              <Button variant="secondary">
                Save as draft
              </Button>
            </div>
          </div>
        )}

        {activeTab === 'display' && (
          <div style={{ padding: 'var(--space-12)', display: 'grid', gap: 'var(--space-8)' }}>
            <div>
              <h3 style={{ font: 'var(--font-title)', marginBottom: 'var(--space-4)' }}>Status Badges</h3>
              <div style={{ display: 'flex', gap: 'var(--space-4)', flexWrap: 'wrap' }}>
                <Badge tone="success">Approved</Badge>
                <Badge tone="warning">Pending review</Badge>
                <Badge tone="danger">Needs revision</Badge>
                <Badge tone="info">In progress</Badge>
              </div>
            </div>

            <div>
              <h3 style={{ font: 'var(--font-title)', marginBottom: 'var(--space-4)' }}>Button Variants</h3>
              <div style={{ display: 'flex', gap: 'var(--space-4)', flexWrap: 'wrap' }}>
                <Button variant="primary" size="md">Primary</Button>
                <Button variant="secondary" size="md">Secondary</Button>
                <Button variant="tertiary" size="md">Tertiary</Button>
                <Button variant="danger" size="md">Danger</Button>
              </div>
            </div>

            <div>
              <h3 style={{ font: 'var(--font-title)', marginBottom: 'var(--space-4)' }}>Icon Buttons</h3>
              <div style={{ display: 'flex', gap: 'var(--space-4)' }}>
                <IconButton>
                  <Plus size={16} />
                </IconButton>
                <IconButton>
                  <ChevronDown size={16} />
                </IconButton>
                <IconButton>
                  <Trash2 size={16} />
                </IconButton>
              </div>
            </div>
          </div>
        )}
      </Card>

      {/* Design Tokens Reference */}
      <Card style={{ marginBottom: 'var(--space-16)' }}>
        <div style={{ padding: 'var(--space-12)' }}>
          <h2 style={{ font: 'var(--font-h2)', marginBottom: 'var(--space-8)' }}>Design Tokens</h2>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-8)' }}>
            <div>
              <h3 style={{ font: 'var(--font-label)', textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', marginBottom: 'var(--space-3)' }}>
                Colors
              </h3>
              <div style={{ display: 'grid', gap: 'var(--space-3)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}>
                  <div style={{ width: '24px', height: '24px', background: 'var(--brand)', borderRadius: 'var(--radius-sm)' }} />
                  <span style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)' }}>--brand</span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}>
                  <div style={{ width: '24px', height: '24px', background: 'var(--success)', borderRadius: 'var(--radius-sm)' }} />
                  <span style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)' }}>--success</span>
                </div>
              </div>
            </div>

            <div>
              <h3 style={{ font: 'var(--font-label)', textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)', marginBottom: 'var(--space-3)' }}>
                Spacing
              </h3>
              <div style={{ display: 'grid', gap: 'var(--space-3)' }}>
                <div style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)' }}>
                  space-4: 8px
                </div>
                <div style={{ font: 'var(--font-caption)', color: 'var(--text-secondary)' }}>
                  space-8: 16px
                </div>
              </div>
            </div>
          </div>
        </div>
      </Card>

      {/* Dialog Example */}
      <Dialog
        open={isOpen}
        onOpenChange={setIsOpen}
        title="Confirm order submission"
      >
        <p style={{ font: 'var(--font-body)', marginBottom: 'var(--space-8)', color: 'var(--text-secondary)' }}>
          This will create a new order and send it to the production queue.
        </p>
        <div style={{ display: 'flex', gap: 'var(--space-4)', justifyContent: 'flex-end' }}>
          <Button
            variant="secondary"
            onClick={() => setIsOpen(false)}
          >
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={() => setIsOpen(false)}
          >
            Confirm
          </Button>
        </div>
      </Dialog>

      {/* Info text */}
      <p style={{ font: 'var(--font-caption)', color: 'var(--text-tertiary)', textAlign: 'center' }}>
        ✨ This example component demonstrates all the major design system patterns.
      </p>
    </div>
  )
}

export default DesignSystemExample
