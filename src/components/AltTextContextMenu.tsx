/**
 * Context menu for adding/editing alt-text on images.
 *
 * Appears as a fixed-position dialog at the right-click location.
 * Closes on Escape, outside click, or after save/cancel.
 */

import React, { useState, useEffect, useRef } from 'react'
import './AltTextContextMenu.css'

export interface AltTextContextMenuProps {
  /** X coordinate in screen space (pixels from left) */
  x: number
  /** Y coordinate in screen space (pixels from top) */
  y: number
  /** PDF object ID for this image */
  objectId: number
  /** Initial alt-text value */
  initialAltText?: string
  /** Initial decorative state */
  initialIsDecorative?: boolean
  /** Called with (altText, isDecorative) on save */
  onSave: (altText: string, isDecorative: boolean) => Promise<void>
  /** Called on cancel or close */
  onCancel: () => void
}

/**
 * Context menu for alt-text editing.
 *
 * Renders at fixed position (x, y) with:
 * - Text input for alt-text content
 * - Checkbox for "Mark as Decorative"
 * - Save and Cancel buttons
 * - Auto-close on Escape or outside click
 */
export function AltTextContextMenu({
  x,
  y,
  objectId,
  initialAltText = '',
  initialIsDecorative = false,
  onSave,
  onCancel,
}: AltTextContextMenuProps) {
  const [altText, setAltText] = useState(initialAltText)
  const [isDecorative, setIsDecorative] = useState(initialIsDecorative)
  const [isSaving, setIsSaving] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  // Handle Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onCancel()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [onCancel])

  // Handle outside click
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onCancel()
      }
    }

    // Delay to avoid immediate close from the original context menu event
    const timeoutId = setTimeout(() => {
      document.addEventListener('click', handleClick)
    }, 100)

    return () => {
      clearTimeout(timeoutId)
      document.removeEventListener('click', handleClick)
    }
  }, [onCancel])

  const handleSave = async () => {
    try {
      setIsSaving(true)
      await onSave(altText, isDecorative)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <div
      ref={menuRef}
      className="alt-text-context-menu"
      style={{
        position: 'fixed',
        left: `${x}px`,
        top: `${y}px`,
      }}
    >
      <div className="alt-text-menu-content">
        <div className="alt-text-menu-header">
          <h3 className="alt-text-menu-title">Image Alt-Text</h3>
          <span className="alt-text-menu-id">#{objectId}</span>
        </div>

        <label className="alt-text-input-group">
          <span className="alt-text-input-label">Description</span>
          <textarea
            className="alt-text-input"
            value={altText}
            onChange={(e) => setAltText(e.target.value)}
            placeholder="Describe the image content..."
            disabled={isDecorative}
            autoFocus
            rows={3}
          />
        </label>

        <label className="alt-text-checkbox-label">
          <input
            type="checkbox"
            className="alt-text-checkbox"
            checked={isDecorative}
            onChange={(e) => setIsDecorative(e.target.checked)}
          />
          <span>Mark as decorative (skip in screen readers)</span>
        </label>

        <div className="alt-text-menu-actions">
          <button
            className="alt-text-btn alt-text-btn--cancel"
            onClick={onCancel}
            disabled={isSaving}
          >
            Cancel
          </button>
          <button
            className="alt-text-btn alt-text-btn--save"
            onClick={handleSave}
            disabled={isSaving}
          >
            {isSaving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  )
}
