import { useEffect, useRef } from 'react'
import type { WindowCommand } from '../../shared/ipc'
import type { IslandAction } from '../../shared/island-activity'
import type { VoiceKey } from '../../shared/voice-shortcut'

interface Navigation {
  newBot(): void
  search(): void
  network(): void
  sidebar(): void
  marketplace(command: Extract<WindowCommand, { kind: 'openMarketplace' }>): void
  voice(key: VoiceKey): void
  island(action: IslandAction): void
  reveal?(ref: string, connectorId?: string): void
  operationSettings?(page: 'updates' | 'computers' | 'channels'): void
}

/** Window navigation stays separate from host mutations and panel rendering. */
export function navigateCommand(command: WindowCommand, navigation: Navigation) {
  switch (command.kind) {
    case 'islandSettings': navigation.operationSettings?.(command.page); break
    case 'islandReveal': navigation.reveal?.(command.ref, command.connectorId); break
    case 'newBot': navigation.newBot(); break
    case 'search': navigation.search(); break
    case 'networkSettings': navigation.network(); break
    case 'toggleSidebar': navigation.sidebar(); break
    case 'openMarketplace': navigation.marketplace(command); break
    case 'voiceKey': navigation.voice(command.key); break
    case 'islandAction':
      if (['openFlow', 'openChannels', 'openDiagnostics'].includes(command.action.kind)) navigation.island(command.action)
      break
  }
}

export function useChatNavigation(navigation: Navigation) {
  useEffect(() => window.codync.app.onCommand((command) => navigateCommand(command, navigation)))
}

/**
 * The view Back returns to: the latest earlier view in a sheet's own history. `history` holds only
 * views already left, so reaching its start means Back should leave the sheet entirely.
 */
export function back_target<T>(current: T, history: readonly T[]): T | null {
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i] !== current) return history[i] ?? null
  }
  return null
}

/** A back handler returns true when it consumed the gesture: an open thread closes before the
 * selected chat changes. Handlers are tried newest first, so the topmost sheet goes back. */
type BackHandler = { handler: () => boolean; priority: number }
const backHandlers = new Set<BackHandler>()

/** Higher priority goes first: an open thread (10) closes before the chat changes (0). */
export function registerBackHandler(handler: () => boolean, priority = 0) {
  const entry = { handler, priority }
  backHandlers.add(entry)
  return () => { backHandlers.delete(entry) }
}

export function goBack(): boolean {
  for (const { handler } of [...backHandlers].sort((a, b) => b.priority - a.priority)) {
    if (handler()) return true
  }
  return false
}

/**
 * Back inside a sheet: an open subview (`first`) closes before the sheet's own page history, and leaving
 * the first page closes the sheet. Higher than the chat's handler, so the sheet goes back before the chat.
 */
export function useBackWithHistory<T>(current: T, setCurrent: (value: T) => void, exit: () => void, first?: () => boolean, priority = 20) {
  const history = useRef<T[]>([])
  const last = useRef(current)
  const before = useRef(first)
  before.current = first
  useEffect(() => {
    if (last.current === current) return
    history.current = [...history.current, last.current]
    last.current = current
  }, [current])
  useEffect(() => registerBackHandler(() => {
    if (before.current?.()) return true
    const target = back_target(current, history.current)
    if (target === null) {
      exit()
      return true
    }
    history.current = history.current.slice(0, -1)
    last.current = target
    setCurrent(target)
    return true
  }, priority), [current, setCurrent, exit, priority])
}

/** Back from a thread goes to the chat or thread it came from: mouse back, Alt+←, ⌘[, UI arrow. */
export function useBackGesture() {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.altKey && event.key === 'ArrowLeft') || (event.metaKey && event.key === '[')) {
        if (goBack()) { event.preventDefault(); event.stopPropagation() }
      }
    }
    const onMouse = (event: MouseEvent) => {
      if (event.button !== 3) return
      if (goBack()) { event.preventDefault(); event.stopPropagation() }
    }
    window.addEventListener('keydown', onKey, true)
    window.addEventListener('mousedown', onMouse, true)
    window.addEventListener('auxclick', onMouse, true)
    return () => {
      window.removeEventListener('keydown', onKey, true)
      window.removeEventListener('mousedown', onMouse, true)
      window.removeEventListener('auxclick', onMouse, true)
    }
  }, [])
}
