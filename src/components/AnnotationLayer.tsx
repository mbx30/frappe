import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PdfAnnotation, PdfAnnotationReply, AnnotationType } from '../types'
import './AnnotationLayer.css'

// ── Shared state hook ───────────────────────────────────────────────────────

const COLORS = [
  { label: 'Yellow', value: '#FFD700' },
  { label: 'Green',  value: '#4CAF50' },
  { label: 'Blue',   value: '#2196F3' },
  { label: 'Red',    value: '#F44336' },
]

const MIN_FRACTION = 0.005

interface DraftRect { x0: number; y0: number; x1: number; y1: number }

export interface AnnotationState {
  activeTool: AnnotationType | null
  setActiveTool: React.Dispatch<React.SetStateAction<AnnotationType | null>>
  activeColor: string
  setActiveColor: React.Dispatch<React.SetStateAction<string>>
  annotations: PdfAnnotation[]
  pageAnnotations: PdfAnnotation[]
  draft: DraftRect | null
  pendingNoteRect: DraftRect | null
  setPendingNoteRect: React.Dispatch<React.SetStateAction<DraftRect | null>>
  selectedAnnotation: PdfAnnotation | null
  setSelectedAnnotation: React.Dispatch<React.SetStateAction<PdfAnnotation | null>>
  replies: PdfAnnotationReply[]
  editingAnnotation: PdfAnnotation | null
  setEditingAnnotation: React.Dispatch<React.SetStateAction<PdfAnnotation | null>>
  overlayRef: React.RefObject<HTMLDivElement>
  dragging: React.MutableRefObject<boolean>
  filePath: string
  pageIndex: number
  pageWidthPts: number
  pageHeightPts: number
  fractionFromEvent: (clientX: number, clientY: number) => { x: number; y: number } | null
  handleMouseDown: (e: React.MouseEvent) => void
  handleMouseMove: (e: React.MouseEvent) => void
  commitDraft: () => void
  saveNote: (text: string) => Promise<void>
  openAnnotation: (ann: PdfAnnotation) => Promise<void>
  deleteAnnotation: (id: number) => Promise<void>
  addReply: (content: string) => Promise<void>
  saveEdit: (text: string) => Promise<void>
}

export function useAnnotations(filePath: string, pageIndex: number, pageWidthPts: number, pageHeightPts: number): AnnotationState {
  const [activeTool, setActiveTool] = useState<AnnotationType | null>(null)
  const [activeColor, setActiveColor] = useState(COLORS[0].value)
  const [annotations, setAnnotations] = useState<PdfAnnotation[]>([])
  const [draft, setDraft] = useState<DraftRect | null>(null)
  const [pendingNoteRect, setPendingNoteRect] = useState<DraftRect | null>(null)
  const [selectedAnnotation, setSelectedAnnotation] = useState<PdfAnnotation | null>(null)
  const [replies, setReplies] = useState<PdfAnnotationReply[]>([])
  const [editingAnnotation, setEditingAnnotation] = useState<PdfAnnotation | null>(null)
  const overlayRef = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)

  useEffect(() => {
    if (!filePath) return
    invoke<PdfAnnotation[]>('pdf_annotations_list', { filePath })
      .then(setAnnotations)
      .catch(() => {})
  }, [filePath])

  const fractionFromEvent = useCallback((clientX: number, clientY: number) => {
    const el = overlayRef.current
    if (!el) return null
    const rect = el.getBoundingClientRect()
    if (rect.width === 0 || rect.height === 0) return null
    return {
      x: Math.min(1, Math.max(0, (clientX - rect.left) / rect.width)),
      y: Math.min(1, Math.max(0, (clientY - rect.top) / rect.height)),
    }
  }, [])

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!activeTool) return
    const f = fractionFromEvent(e.clientX, e.clientY)
    if (!f) return
    dragging.current = true
    setDraft({ x0: f.x, y0: f.y, x1: f.x, y1: f.y })
  }, [activeTool, fractionFromEvent])

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragging.current) return
    const f = fractionFromEvent(e.clientX, e.clientY)
    if (!f) return
    setDraft((d) => d ? { ...d, x1: f.x, y1: f.y } : null)
  }, [fractionFromEvent])

  const commitDraft = useCallback(() => {
    if (!dragging.current) return
    dragging.current = false
    setDraft((d) => {
      if (d && pageWidthPts > 0 && pageHeightPts > 0) {
        const fx0 = Math.min(d.x0, d.x1)
        const fy0 = Math.min(d.y0, d.y1)
        const fw = Math.abs(d.x1 - d.x0)
        const fh = Math.abs(d.y1 - d.y0)
        if (fw >= MIN_FRACTION && fh >= MIN_FRACTION) {
          if (activeTool === 'note') {
            setPendingNoteRect(d)
          } else {
            invoke<PdfAnnotation>('pdf_annotation_add', {
              filePath,
              page: pageIndex,
              annotationType: activeTool,
              x: fx0 * pageWidthPts,
              y: fy0 * pageHeightPts,
              width: fw * pageWidthPts,
              height: fh * pageHeightPts,
              color: activeColor,
              content: '',
            })
              .then((ann) => setAnnotations((prev) => [...prev, ann]))
              .catch(() => {})
          }
        }
      }
      return null
    })
  }, [activeTool, filePath, pageIndex, pageWidthPts, pageHeightPts, activeColor])

  const saveNote = useCallback(async (text: string) => {
    if (!pendingNoteRect || pageWidthPts === 0 || pageHeightPts === 0) {
      setPendingNoteRect(null)
      return
    }
    const d = pendingNoteRect
    const fx0 = Math.min(d.x0, d.x1)
    const fy0 = Math.min(d.y0, d.y1)
    const fw = Math.abs(d.x1 - d.x0)
    const fh = Math.abs(d.y1 - d.y0)
    try {
      const ann = await invoke<PdfAnnotation>('pdf_annotation_add', {
        filePath,
        page: pageIndex,
        annotationType: 'note',
        x: fx0 * pageWidthPts,
        y: fy0 * pageHeightPts,
        width: fw * pageWidthPts,
        height: fh * pageHeightPts,
        color: activeColor,
        content: text,
      })
      setAnnotations((prev) => [...prev, ann])
    } catch {
      // ignore
    }
    setPendingNoteRect(null)
  }, [pendingNoteRect, filePath, pageIndex, pageWidthPts, pageHeightPts, activeColor])

  const openAnnotation = useCallback(async (ann: PdfAnnotation) => {
    setSelectedAnnotation(ann)
    try {
      const r = await invoke<PdfAnnotationReply[]>('pdf_annotation_replies_list', { annotationId: ann.id })
      setReplies(r)
    } catch {
      setReplies([])
    }
  }, [])

  const deleteAnnotation = useCallback(async (id: number) => {
    try {
      await invoke('pdf_annotation_delete', { id })
      setAnnotations((prev) => prev.filter((a) => a.id !== id))
      setSelectedAnnotation(null)
    } catch {
      // ignore
    }
  }, [])

  const addReply = useCallback(async (content: string) => {
    if (!selectedAnnotation) return
    try {
      const reply = await invoke<PdfAnnotationReply>('pdf_annotation_reply_add', {
        annotationId: selectedAnnotation.id,
        content,
      })
      setReplies((prev) => [...prev, reply])
    } catch {
      // ignore
    }
  }, [selectedAnnotation])

  const saveEdit = useCallback(async (text: string) => {
    if (!editingAnnotation) return
    try {
      const updated = await invoke<PdfAnnotation>('pdf_annotation_update', {
        id: editingAnnotation.id,
        content: text,
      })
      setAnnotations((prev) => prev.map((a) => a.id === updated.id ? updated : a))
    } catch {
      // ignore
    }
    setEditingAnnotation(null)
    setSelectedAnnotation(null)
  }, [editingAnnotation])

  const pageAnnotations = annotations.filter((a) => a.page === pageIndex)

  return {
    activeTool, setActiveTool,
    activeColor, setActiveColor,
    annotations, pageAnnotations,
    draft, pendingNoteRect, setPendingNoteRect,
    selectedAnnotation, setSelectedAnnotation,
    replies,
    editingAnnotation, setEditingAnnotation,
    overlayRef, dragging,
    filePath, pageIndex, pageWidthPts, pageHeightPts,
    fractionFromEvent,
    handleMouseDown, handleMouseMove, commitDraft,
    saveNote, openAnnotation, deleteAnnotation, addReply, saveEdit,
  }
}

// ── Sub-components ──────────────────────────────────────────────────────────

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

// ── Public components ───────────────────────────────────────────────────────

/** Toolbar row: tool buttons + color swatches. Render above the page image. */
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
    overlayRef, handleMouseDown, handleMouseMove, commitDraft,
    saveNote, openAnnotation, deleteAnnotation, addReply, saveEdit,
  } = state

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
    </>
  )
}
