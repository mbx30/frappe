/**
 * Alt-text session state management using React Context + useReducer.
 *
 * Tracks unsaved alt-text edits per file and object, with dirty flag tracking
 * to warn users when closing with unsaved changes.
 */

import { createContext, useContext, useReducer, ReactNode, useCallback } from 'react'

/**
 * Represents the alt-text and decorative status for a single image.
 */
export interface AltTextEntry {
  objectId: number
  altText: string
  isDecorative: boolean
  isDirty: boolean
  savedAltText: string
  savedIsDecorative: boolean
}

/**
 * Session state for a single PDF file.
 */
export interface AltTextSession {
  entries: Map<number, AltTextEntry>
  hasUnsavedChanges: boolean
}

/**
 * Global store state: maps file paths to their session data.
 */
export interface AltTextStoreState {
  sessions: Map<string, AltTextSession>
}

/**
 * Action types for the reducer.
 */
type AltTextAction =
  | {
      type: 'UPDATE_ALT_TEXT'
      filePath: string
      objectId: number
      altText: string
      isDecorative: boolean
    }
  | {
      type: 'MARK_SAVED'
      filePath: string
      objectId: number
    }
  | {
      type: 'CLEAR_SESSION'
      filePath: string
    }
  | {
      type: 'INIT_SESSION'
      filePath: string
      entries: Array<{ objectId: number; altText: string; isDecorative: boolean }>
    }

/**
 * Reducer function for managing alt-text session state.
 */
function altTextReducer(state: AltTextStoreState, action: AltTextAction): AltTextStoreState {
  const sessions = new Map(state.sessions)

  switch (action.type) {
    case 'UPDATE_ALT_TEXT': {
      let session = sessions.get(action.filePath)
      if (!session) {
        session = { entries: new Map(), hasUnsavedChanges: false }
      }

      const entries = new Map(session.entries)
      const existing = entries.get(action.objectId) || {
        objectId: action.objectId,
        savedAltText: '',
        savedIsDecorative: false,
        altText: '',
        isDecorative: false,
        isDirty: false,
      }

      entries.set(action.objectId, {
        ...existing,
        altText: action.altText,
        isDecorative: action.isDecorative,
        isDirty: true,
      })

      sessions.set(action.filePath, {
        entries,
        hasUnsavedChanges: Array.from(entries.values()).some((e) => e.isDirty),
      })
      break
    }

    case 'MARK_SAVED': {
      const session = sessions.get(action.filePath)
      if (!session) break

      const entries = new Map(session.entries)
      const entry = entries.get(action.objectId)
      if (entry) {
        entries.set(action.objectId, {
          ...entry,
          isDirty: false,
          savedAltText: entry.altText,
          savedIsDecorative: entry.isDecorative,
        })
      }

      sessions.set(action.filePath, {
        entries,
        hasUnsavedChanges: Array.from(entries.values()).some((e) => e.isDirty),
      })
      break
    }

    case 'CLEAR_SESSION': {
      sessions.delete(action.filePath)
      break
    }

    case 'INIT_SESSION': {
      const entries = new Map<number, AltTextEntry>()
      action.entries.forEach((e) => {
        entries.set(e.objectId, {
          objectId: e.objectId,
          altText: e.altText,
          isDecorative: e.isDecorative,
          isDirty: false,
          savedAltText: e.altText,
          savedIsDecorative: e.isDecorative,
        })
      })

      sessions.set(action.filePath, {
        entries,
        hasUnsavedChanges: false,
      })
      break
    }
  }

  return { sessions }
}

/**
 * Context for alt-text session store.
 */
const AltTextStoreContext = createContext<{
  state: AltTextStoreState
  dispatch: (action: AltTextAction) => void
} | null>(null)

/**
 * Provider component for alt-text store.
 */
export function AltTextStoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(altTextReducer, { sessions: new Map() })

  return (
    <AltTextStoreContext.Provider value={{ state, dispatch }}>
      {children}
    </AltTextStoreContext.Provider>
  )
}

/**
 * Hook to use the alt-text store.
 */
export function useAltTextStore() {
  const context = useContext(AltTextStoreContext)
  if (!context) {
    throw new Error('useAltTextStore must be used within AltTextStoreProvider')
  }

  const { state, dispatch } = context

  const updateAltText = useCallback(
    (filePath: string, objectId: number, altText: string, isDecorative: boolean) => {
      dispatch({
        type: 'UPDATE_ALT_TEXT',
        filePath,
        objectId,
        altText,
        isDecorative,
      })
    },
    []
  )

  const markSaved = useCallback((filePath: string, objectId: number) => {
    dispatch({
      type: 'MARK_SAVED',
      filePath,
      objectId,
    })
  }, [])

  const clearSession = useCallback((filePath: string) => {
    dispatch({
      type: 'CLEAR_SESSION',
      filePath,
    })
  }, [])

  const initSession = useCallback(
    (filePath: string, entries: Array<{ objectId: number; altText: string; isDecorative: boolean }>) => {
      dispatch({
        type: 'INIT_SESSION',
        filePath,
        entries,
      })
    },
    []
  )

  const hasUnsavedChanges = useCallback(
    (filePath: string): boolean => {
      const session = state.sessions.get(filePath)
      return session?.hasUnsavedChanges ?? false
    },
    [state.sessions]
  )

  const getSession = useCallback(
    (filePath: string): AltTextSession | undefined => {
      return state.sessions.get(filePath)
    },
    [state.sessions]
  )

  const getUnsavedChanges = useCallback(
    (filePath: string): Array<{ objectId: number; altText: string; isDecorative: boolean }> => {
      const session = state.sessions.get(filePath)
      if (!session) return []

      return Array.from(session.entries.values())
        .filter((e) => e.isDirty)
        .map((e) => ({
          objectId: e.objectId,
          altText: e.altText,
          isDecorative: e.isDecorative,
        }))
    },
    [state.sessions]
  )

  return {
    state,
    updateAltText,
    markSaved,
    clearSession,
    initSession,
    hasUnsavedChanges,
    getSession,
    getUnsavedChanges,
  }
}
