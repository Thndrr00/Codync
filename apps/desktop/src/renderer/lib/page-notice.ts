/**
 * A composer notice when the agent has just put a page in reach (OpenBot 0.72.0's "Open page"). The
 * notice keeps the page's own thread, so opening it from a side conversation still goes there.
 */
export interface PageNoticeSource {
  kind?: string
  threadId?: string | null
  data?: {
    artifact?: { id: string; title: string } | null
    text?: string | null
  } | null
}

export interface PageNotice {
  label: string
  /** An artifact id, or a web address. */
  href: string
  title: string
  thread: string | null
}

const WEB_LINK = /https?:\/\/\S+/i

/** Pure decision: the agent's latest page, or null for anything else (a user line, a tool row, no page). */
export function pageNotice(entry: PageNoticeSource | null): PageNotice | null {
  if (!entry || entry.kind !== 'agent') return null
  const card = entry.data?.artifact
  if (card?.id) return { label: 'Open page', href: card.id, title: card.title || 'Page', thread: entry.threadId ?? null }
  const url = entry.data?.text?.match(WEB_LINK)?.[0]
  return url ? { label: 'Open page', href: url, title: url, thread: entry.threadId ?? null } : null
}
