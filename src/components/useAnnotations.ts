import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PdfAnnotation, PdfAnnotationReply, AnnotationType } from '../types'

export const COLORS = [
  { label: 'Yellow', value: '#FFD700' },
  { label: 'Green',  value: '#4CAF50' },
  { label: 'Blue',   value: '#2196F3' },
  { label: 'Red',    value: '#F44336' },
]

export const MIN_FRACTION = 0.005

export interface DraftRect { x0: number; y0: number; x1: number; y1: number }

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
  overlayRef: React.RefObject<HTMLDivElement | null>
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
