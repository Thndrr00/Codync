/**
 * Ported from OpenBot 0.75: "Files that you open next to a chat now open again after OpenBot
 * restarts. The switch 'Restore the last workspace on launch' in Settings now controls this."
 *
 * The device pref holds ids only — never a file name, its bytes, a page address or any other data a
 * secret could hide in. A saved set is scoped to one chat on one computer in one window, and is only
 * brought back while "Restore the last workspace on launch" is on.
 */

/** No chat keeps an unbounded strip of tabs. */
export const MAX_OPEN_TABS = 8

export interface ChatRef {
  computerId: string
  botId: string
}

export type SavedTab =
  | { kind: 'file'; entryId: string; fileId: string }
  | { kind: 'artifact'; artifactId: string }

export interface SavedChatTabs {
  /** The window that saved these; a different window never adopts them. */
  window: string
  computerId: string
  botId: string
  tabs: SavedTab[]
  active: SavedTab | null
}

export type SavedTabsByChat = Record<string, SavedChatTabs>

export const chatKey = ({ computerId, botId }: ChatRef): string => `${computerId}/${botId}`

const MAX_ID = 200
/** Host-generated ids are plain tokens. Paths, URLs, quotes and control characters are foreign data. */
const unsafeId = /[\u0000-\u001f\u007f/\\@:"'\s]/

function safeId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_ID && !unsafeId.test(value)
}

function isSavedTab(value: unknown): value is SavedTab {
  if (!value || typeof value !== 'object') return false
  const tab = value as Record<string, unknown>
  if (tab.kind === 'file') return safeId(tab.entryId) && safeId(tab.fileId)
  if (tab.kind === 'artifact') return safeId(tab.artifactId)
  return false
}

const keyOf = (tab: SavedTab): string => tab.kind === 'file' ? `f:${tab.entryId}/${tab.fileId}` : `a:${tab.artifactId}`

/** Whether two saved tabs name the same file or artifact. */
export const sameTab = (a: SavedTab, b: SavedTab): boolean => keyOf(a) === keyOf(b)

/** A file tab is safe only when its entry belongs to this chat and its id is a host id with a real digest. */
export function safeFileTab(
  open: { entryId: string; entry: { id: string; botId: string }; file: { id: string; sha256: string } },
  chat: ChatRef,
): SavedTab | null {
  if (open.entry.botId !== chat.botId || open.entryId !== open.entry.id) return null
  if (!safeId(open.entryId) || !safeId(open.file.id)) return null
  if (typeof open.file.sha256 !== 'string' || !/^[0-9a-f]{64}$/i.test(open.file.sha256)) return null
  return { kind: 'file', entryId: open.entryId, fileId: open.file.id }
}

/** An artifact tab keeps just the artifact's id, never its title or contents. */
export function safeArtifactTab(id: unknown): SavedTab | null {
  return safeId(id) ? { kind: 'artifact', artifactId: id } : null
}

/** Drops repeats and keeps the newest `max` tabs, in open order. */
export function capTabs(tabs: readonly SavedTab[], max = MAX_OPEN_TABS): SavedTab[] {
  const unique = new Map<string, SavedTab>()
  for (const tab of tabs) {
    const key = keyOf(tab)
    unique.delete(key)
    unique.set(key, tab)
  }
  return [...unique.values()].slice(-Math.max(0, max))
}

/**
 * The tabs to bring back for `chat`, or null. Nothing is restored while the switch is off, from
 * another computer or chat, from a different window, or from a malformed saved value.
 */
export function restoreSavedTabs(
  saved: SavedTabsByChat,
  chat: ChatRef,
  window: string,
  restoreOnLaunch: boolean,
): SavedChatTabs | null {
  if (!restoreOnLaunch) return null
  const entry = saved[chatKey(chat)]
  if (!entry || typeof entry !== 'object') return null
  if (entry.computerId !== chat.computerId || entry.botId !== chat.botId || entry.window !== window) return null
  if (!Array.isArray(entry.tabs)) return null
  const tabs = capTabs(entry.tabs.filter(isSavedTab))
  if (!tabs.length) return null
  const active = isSavedTab(entry.active) && tabs.some((tab) => sameTab(tab, entry.active!))
    ? entry.active
    : tabs[tabs.length - 1]
  return { window, ...chat, tabs, active }
}

// --- Window identity: sessionStorage is per window; localStorage is shared by this device's windows.

const SESSION_KEY = 'codyncOpenTabsWindow'
const OWNER_KEY = 'codyncOpenTabsWorkspace'
/** A workspace last touched this recently is another live window, not a previous launch. */
const ADOPT_AFTER_MS = 10_000

/** Refreshes the heartbeat so a window opened at the same time knows not to adopt this workspace. */
export function markWindowActive(window: string) {
  try {
    localStorage.setItem(OWNER_KEY, JSON.stringify({ window, at: Date.now() }))
  } catch {
    // A pref we cannot write simply is not saved; the chat keeps working.
  }
}

/** This window's token: its own for the session, adopting the last launch's workspace when fresh. */
export function windowToken(): string {
  try {
    const own = sessionStorage.getItem(SESSION_KEY)
    if (own) return own
    const raw = localStorage.getItem(OWNER_KEY)
    const last = raw ? JSON.parse(raw) as { window?: unknown; at?: unknown } : null
    const token = last && typeof last.window === 'string' && typeof last.at === 'number' && Date.now() - last.at > ADOPT_AFTER_MS
      ? last.window
      : crypto.randomUUID()
    sessionStorage.setItem(SESSION_KEY, token)
    markWindowActive(token)
    return token
  } catch {
    return `window-${Date.now()}`
  }
}
