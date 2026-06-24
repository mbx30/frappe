/// Central registry of keyboard shortcuts used by the PDF viewer and
/// preflight components. Issue #276. Keeping the keys in a single module
/// avoids drift between the help dialog and the runtime handlers.

export type ShortcutAction =
  | 'find'
  | 'save-profile'
  | 'run-profile'
  | 'next-page'
  | 'prev-page'
  | 'first-page'
  | 'last-page'
  | 'fullscreen'
  | 'help'
  | 'open'
  | 'run-preflight'

export interface Shortcut {
  key: string
  /** Modifier: 'any' to mean Cmd or Ctrl, or a specific 'ctrl' | 'meta'. */
  modifier?: 'any' | 'ctrl' | 'meta'
  shift?: boolean
  alt?: boolean
  description: string
  action: ShortcutAction
}

const isMac = typeof navigator !== 'undefined' && /mac/i.test(navigator.platform || '')

function mod() {
  return isMac ? 'meta' : 'ctrl'
}

export function buildShortcuts(): Shortcut[] {
  const m = mod()
  return [
    { key: 'f', modifier: m, description: 'Find text in document', action: 'find' },
    { key: 's', modifier: m, description: 'Save current preflight profile', action: 'save-profile' },
    { key: 'r', modifier: m, description: 'Run preflight profile', action: 'run-profile' },
    { key: 'o', modifier: m, description: 'Open PDF file', action: 'open' },
    { key: 'p', modifier: m, shift: true, description: 'Run full preflight', action: 'run-preflight' },
    { key: 'ArrowRight', description: 'Next page', action: 'next-page' },
    { key: 'ArrowLeft', description: 'Previous page', action: 'prev-page' },
    { key: 'Home', description: 'First page', action: 'first-page' },
    { key: 'End', description: 'Last page', action: 'last-page' },
    { key: 'f', description: 'Toggle fullscreen (no modifier)', action: 'fullscreen' },
    { key: '?', shift: true, description: 'Show shortcut help', action: 'help' },
  ]
}

export function matchesShortcut(e: KeyboardEvent, s: Shortcut): boolean {
  if (e.key.toLowerCase() !== s.key.toLowerCase() && e.key !== s.key) {
    return false
  }
  const wantShift = !!s.shift
  if (e.shiftKey !== wantShift) return false
  if (s.alt && !e.altKey) return false
  if (s.modifier === 'any') {
    if (!(e.ctrlKey || e.metaKey)) return false
  } else if (s.modifier === 'ctrl') {
    if (!e.ctrlKey) return false
  } else if (s.modifier === 'meta') {
    if (!e.metaKey) return false
  }
  return true
}

export interface ShortcutHandlers {
  onFind?: () => void
  onSaveProfile?: () => void
  onRunProfile?: () => void
  onOpen?: () => void
  onRunPreflight?: () => void
  onNextPage?: () => void
  onPrevPage?: () => void
  onFirstPage?: () => void
  onLastPage?: () => void
  onFullscreen?: () => void
  onHelp?: () => void
}

/// Returns a `keydown` handler that dispatches to the supplied
/// `ShortcutHandlers`. The returned function is intended to be attached to
/// `window` while the PDF viewer is mounted.
export function makeKeyDownHandler(handlers: ShortcutHandlers): (e: KeyboardEvent) => void {
  const shortcuts = buildShortcuts()
  return (e: KeyboardEvent) => {
    const target = e.target as HTMLElement | null
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
      if (!(e.key === '?' || (e.key === 'f' && !e.ctrlKey && !e.metaKey))) return
    }
    for (const s of shortcuts) {
      if (matchesShortcut(e, s)) {
        switch (s.action) {
          case 'find': handlers.onFind?.(); break
          case 'save-profile': handlers.onSaveProfile?.(); break
          case 'run-profile': handlers.onRunProfile?.(); break
          case 'open': handlers.onOpen?.(); break
          case 'run-preflight': handlers.onRunPreflight?.(); break
          case 'next-page': handlers.onNextPage?.(); break
          case 'prev-page': handlers.onPrevPage?.(); break
          case 'first-page': handlers.onFirstPage?.(); break
          case 'last-page': handlers.onLastPage?.(); break
          case 'fullscreen': handlers.onFullscreen?.(); break
          case 'help': handlers.onHelp?.(); break
        }
        e.preventDefault()
        return
      }
    }
  }
}

export function formatShortcut(s: Shortcut): string {
  const parts: string[] = []
  if (s.modifier) {
    parts.push(s.modifier === 'meta' ? '⌘' : 'Ctrl')
  }
  if (s.shift) parts.push('Shift')
  if (s.alt) parts.push('Alt')
  parts.push(s.key === ' ' ? 'Space' : s.key)
  return parts.join('+')
}
