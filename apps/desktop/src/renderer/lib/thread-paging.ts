/** The archived-thread drawer is paged: a full page means another one may exist. */
export const THREAD_PAGE_SIZE = 50

export interface ThreadPage<T> {
  /** The rows to show, newest first, with the older page merged in. */
  threads: T[]
  /** Whether another "Show older threads" tap can find anything new. */
  hasMore: boolean
}

/**
 * Merges one host page into the rows already shown. An older host ignores `before` and keeps
 * sending the same 50 rows; when nothing new arrives, the list stops offering another page so
 * the button never spins forever. Messages are never removed here, so saved threads keep theirs.
 */
export function mergeThreadPage<T extends { id: string; updatedAt: number }>(
  existing: T[], incoming: T[], pageSize = THREAD_PAGE_SIZE,
): ThreadPage<T> {
  const known = new Set(existing.map((thread) => thread.id))
  const added = incoming.filter((thread) => !known.has(thread.id))
  return {
    threads: [...existing, ...added].sort((a, b) => b.updatedAt - a.updatedAt),
    hasMore: incoming.length >= pageSize && added.length > 0,
  }
}

/** The cursor for the next page: the oldest row already shown. */
export function threadPageCursor<T>(threads: T[]): string | null {
  return threads.length ? (threads[threads.length - 1] as { id: string }).id : null
}
