import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PdfAnnotation, PdfAnnotationReply, AnnotationType } from '../types'
import { type AnnotationState, COLORS } from './useAnnotations'
import { AltTextContextMenu } from './AltTextContextMenu'
import { useAltTextStore } from '../store/altTextStore'
import './AnnotationLayer.css'

function NoteDialog({ initial, onSave, onCancel }: { initial: string; onSave: (t: string) => void; onCancel: () => void }) {
  const [text, setText] = useState(initial)
  return (
    <div className="annot-dialog-backdrop" role="dialog" aria-modal="true" aria-label="Note">
      <div className="annot-dialog">
        <h3 className="annot-dialog-title">Note</h3>
        <textarea
          className="annot-dialog-textarea"
          value={text}
          onChange={(e) => setText(e.target.value)}
          rows={4}
          autoFocus
          aria-label="Note text"
        />
        <div className="annot-dialog-actions">
          <button className="btn btn-secondary" onClick={onCancel}>Cancel</button>
          <button className="btn btn-primary" onClick={() => onSave(text)}>Save</button>
        </div>
      </div>
    </div>
  )
}

function RepliesDialog({
  annotation, replies, onAddReply, onClose, onDelete, onEdit,
}: {
  annotation: PdfAnnotation
  replies: PdfAnnotationReply[]
  onAddReply: (c: string) => void
  onClose: () => void
  onDelete: () => void
  onEdit: () => void
}) {
  const [replyText, setReplyText] = useState('')
  return (
    <div className="annot-dialog-backdrop" role="dialog" aria-modal="true" aria-label="Annotation details">
      <div className="annot-dialog annot-dialog--wide">
        <div className="annot-dialog-header">
          <h3 className="annot-dialog-title">{annotation.annotation_type.charAt(0).toUpperCase() + annotation.annotation_type.slice(1)}</h3>
          <button className="annot-close-btn" onClick={onClose} aria-label="Close">✕</button>
        </div>
        {annotation.content && (
          <p className="annot-dialog-note-text">{annotation.content}</p>
        )}
        <div className="annot-dialog-actions annot-dialog-actions--left">
          <button className="btn btn-secondary" onClick={onEdit}>Edit</button>
          <button className="btn btn-danger" onClick={onDelete}>Delete</button>
        </div>
        {replies.length > 0 && (
          <div className="annot-replies">
            {replies.map((r) => (
              <div key={r.id} className="annot-reply">
                <span className="annot-reply-text">{r.content}</span>
                <span className="annot-reply-date">{r.created_at.slice(0, 10)}</span>
              </div>
            ))}
          </div>
        )}
        <div className="annot-reply-form">
          <textarea
            className="annot-dialog-textarea"
            value={replyText}
            onChange={(e) => setReplyText(e.target.value)}
            rows={2}
            placeholder="Add a reply…"
            aria-label="Reply text"
          />
          <button
            className="btn btn-primary"
            disabled={!replyText.trim()}
            onClick={() => { onAddReply(replyText); setReplyText('') }}
          >
            Reply
          </button>
        </div>
      </div>
    </div>
  )
}

export function AnnotationToolbar({ state }: { state: AnnotationState }) {
  const { activeTool, setActiveTool, activeColor, setActiveColor, pageAnnotations } = state
  return (
    <div className="annot-toolbar" role="toolbar" aria-label="Annotation tools">
      {(['highlight', 'underline', 'strikethrough', 'note'] as AnnotationType[]).map((tool) => (
        <button
          key={tool}
          className={`annot-tool-btn${activeTool === tool ? ' annot-tool-btn--active' : ''}`}
          aria-pressed={activeTool === tool}
          aria-label={tool.charAt(0).toUpperCase() + tool.slice(1)}
          title={tool.charAt(0).toUpperCase() + tool.slice(1)}
          onClick={() => setActiveTool((t) => t === tool ? null : tool)}
        >
          {tool === 'highlight' ? '🖊' : tool === 'underline' ? 'U̲' : tool === 'strikethrough' ? 'S̶' : '📝'}
        </button>
      ))}
      <div className="annot-color-row" role="group" aria-label="Annotation color">
        {COLORS.map((c) => (
          <button
            key={c.value}
            className={`annot-color-swatch${activeColor === c.value ? ' annot-color-swatch--active' : ''}`}
            style={{ background: c.value }}
            aria-label={c.label}
            aria-pressed={activeColor === c.value}
            title={c.label}
            onClick={() => setActiveColor(c.value)}
          />
        ))}
      </div>
      {pageAnnotations.length > 0 && (
        <span className="annot-count" aria-live="polite">
          {pageAnnotations.length} annotation{pageAnnotations.length !== 1 ? 's' : ''}
        </span>
      )}
    </div>
  )
}

/** Overlay: renders annotation boxes and handles mouse drawing.
 *  Must be placed inside a `position:relative` container sized to the page image. */
export function AnnotationOverlay({ state }: { state: AnnotationState }) {
  const {
    activeTool, pageAnnotations, draft, pageWidthPts, pageHeightPts,
    pendingNoteRect, setPendingNoteRect,
    selectedAnnotation, setSelectedAnnotation,
    replies, editingAnnotation, setEditingAnnotation,
    pendingAltTextMenu, setPendingAltTextMenu,
    overlayRef, handleMouseDown, handleMouseMove, commitDraft,
    saveNote, openAnnotation, deleteAnnotation, addReply, saveEdit,
    filePath,
  } = state

  const { updateAltText, markSaved } = useAltTextStore()

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    // Generate a simple object ID based on position (placeholder for real image detection)
    // In a real implementation, this would map pixel coordinates to actual PDF image XObject IDs
    const objectId = Math.floor(Math.random() * 10000)
    setPendingAltTextMenu({
      x: e.clientX,
      y: e.clientY,
      objectId,
      initialAltText: '',
      initialIsDecorative: false,
    })
  }

  const handleSaveAltText = async (altText: string, isDecorative: boolean) => {
    if (!pendingAltTextMenu) return
    try {
      // Update session store immediately
      updateAltText(filePath, pendingAltTextMenu.objectId, altText, isDecorative)
      // Save to backend
      await invoke('set_alt_text', {
        filePath,
        objectId: pendingAltTextMenu.objectId,
        altText,
        isDecorative,
      })
      markSaved(filePath, pendingAltTextMenu.objectId)
    } catch (error) {
      console.error('Failed to save alt-text:', error)
    }
    setPendingAltTextMenu(null)
  }

  const draftStyle = draft
    ? {
        left: `${Math.min(draft.x0, draft.x1) * 100}%`,
        top: `${Math.min(draft.y0, draft.y1) * 100}%`,
        width: `${Math.abs(draft.x1 - draft.x0) * 100}%`,
        height: `${Math.abs(draft.y1 - draft.y0) * 100}%`,
      }
    : null

  return (
    <>
      <div
        ref={overlayRef}
        className={`annot-overlay${activeTool ? ' annot-overlay--drawing' : ''}`}
        role="application"
        aria-label="Annotation canvas"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={commitDraft}
        onMouseLeave={commitDraft}
        onContextMenu={handleContextMenu}
      >
        {pageAnnotations.map((ann) => {
          if (pageWidthPts === 0 || pageHeightPts === 0) return null
          const style: React.CSSProperties = {
            left: `${(ann.x / pageWidthPts) * 100}%`,
            top: `${(ann.y / pageHeightPts) * 100}%`,
            width: `${(ann.width / pageWidthPts) * 100}%`,
            height: `${(ann.height / pageHeightPts) * 100}%`,
          }
          return (
            <div
              key={ann.id}
              className={`annot-box annot-box--${ann.annotation_type}`}
              style={{ ...style, '--annot-color': ann.color } as React.CSSProperties}
              role="button"
              tabIndex={0}
              aria-label={`${ann.annotation_type} annotation`}
              onClick={(e) => { e.stopPropagation(); openAnnotation(ann) }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.stopPropagation()
                  openAnnotation(ann)
                }
              }}
            >
              {ann.annotation_type === 'note' && (
                <span className="annot-note-pin" aria-hidden="true">📝</span>
              )}
            </div>
          )
        })}
        {draftStyle && (
          <div className="annot-box annot-box--draft" style={draftStyle} aria-hidden="true" />
        )}
      </div>

      {pendingNoteRect && (
        <NoteDialog
          initial=""
          onSave={saveNote}
          onCancel={() => setPendingNoteRect(null)}
        />
      )}
      {editingAnnotation && (
        <NoteDialog
          initial={editingAnnotation.content}
          onSave={saveEdit}
          onCancel={() => setEditingAnnotation(null)}
        />
      )}
      {selectedAnnotation && !editingAnnotation && (
        <RepliesDialog
          annotation={selectedAnnotation}
          replies={replies}
          onAddReply={addReply}
          onClose={() => setSelectedAnnotation(null)}
          onDelete={() => deleteAnnotation(selectedAnnotation.id)}
          onEdit={() => setEditingAnnotation(selectedAnnotation)}
        />
      )}
      {pendingAltTextMenu && (
        <AltTextContextMenu
          x={pendingAltTextMenu.x}
          y={pendingAltTextMenu.y}
          objectId={pendingAltTextMenu.objectId}
          initialAltText={pendingAltTextMenu.initialAltText}
          initialIsDecorative={pendingAltTextMenu.initialIsDecorative}
          onSave={handleSaveAltText}
          onCancel={() => setPendingAltTextMenu(null)}
        />
      )}
    </>
  )
}
