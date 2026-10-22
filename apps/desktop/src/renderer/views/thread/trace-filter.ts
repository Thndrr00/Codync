// The typed filters of "Full conversation" (idea 32, as Android's TraceFilter). Each one is decided
// by the entry's real event type (`kind`, plus the structured `status` / `style` fields), never by
// words in its text. A filter only chooses what shows: it never removes entries from the conversation.

import type { Entry } from '@shared/models'

export type TraceFilterId = 'all' | 'messages' | 'thinking' | 'tools' | 'plans' | 'approvals' | 'problems'

export interface TraceFilterDef {
  id: TraceFilterId
  label: string
}

export const TRACE_FILTERS: TraceFilterDef[] = [
  { id: 'all', label: 'All' },
  { id: 'messages', label: 'Messages' },
  { id: 'thinking', label: 'Thinking' },
  { id: 'tools', label: 'Tools' },
  { id: 'plans', label: 'Plans' },
  { id: 'approvals', label: 'Approvals' },
  { id: 'problems', label: 'Problems' },
]

export function matchesFilter(e: Entry, id: TraceFilterId): boolean {
  switch (id) {
    case 'all':
      return true
    case 'messages':
      return e.kind === 'user' || e.kind === 'agent'
    case 'thinking':
      return e.kind === 'thought'
    case 'tools':
      return e.kind === 'tool'
    case 'plans':
      return e.kind === 'plan'
    case 'approvals':
      return e.kind === 'permission'
    case 'problems':
      return (e.kind === 'tool' && e.data.status === 'failed') || (e.kind === 'notice' && e.data.style === 'error')
  }
}

/** How many of [entries] each filter shows, for the chips' counts. */
export function filterCounts(entries: Entry[]): Record<TraceFilterId, number> {
  const out = {} as Record<TraceFilterId, number>
  for (const f of TRACE_FILTERS) out[f.id] = entries.filter((e) => matchesFilter(e, f.id)).length
  return out
}
