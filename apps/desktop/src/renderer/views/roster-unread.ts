/** The minimal shape this decision needs; imported Entry types are only used at the call site. */
interface RosterEntry {
  id: string
  kind: string
  data: { final?: boolean }
}

/** Chat-visible means a final agent entry, the same rule the shared `isChat` helper uses. */
function isChatEntry(entry: RosterEntry): boolean {
  return entry.kind === 'agent' && entry.data.final === true
}

/** The host payload behind a roster "Mark unread" menu item. */
export interface UnreadAction {
  entryId: string
}

/**
 * The pure decision behind "Mark unread": the host needs a message to start from,
 * so without one there is no menu item to offer.
 */
export function unreadAction(entryId: string | null): UnreadAction | null {
  return entryId ? { entryId } : null
}

/**
 * Which message a roster "Mark unread" starts from: the newest chat-visible agent
 * reply in the cached conversation. The host marks it and everything after it
 * unread; opening the chat clears them again.
 */
export function unreadEntryId(entries: readonly RosterEntry[]): string | null {
  for (let i = entries.length - 1; i >= 0; i--) {
    const entry = entries[i]!
    if (isChatEntry(entry)) return entry.id
  }
  return null
}
