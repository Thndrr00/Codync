import type { IslandBot } from './island'
import type { IslandDiff } from './island'

/** Small presentation snapshots. The host retains files, queues and execution logic. */
export interface IslandWorkspace {
  ref: string
  computerName?: string
  loading: boolean
  error: string | null
  actionError: string | null
  busy: boolean
  artifacts: { id: string; title: string; at: number }[]
  files: { id: string; entryId: string; name: string; at: number }[]
  changes: { entryId: string; at: number; diffs: IslandDiff[] }[]
  queue: { id: string; at: number }[]
  git: { branch: string | null; changes: number | null; error: string | null } | null
  timings: { routineId: string; estimateMs: number; samples: number }[]
  day: { messages: number; replies: number; tools: number; fileEdits: number } | null
}

export type IslandScope = 'all' | 'bot' | 'project'

/** Paths from different computers never identify the same project. */
export function inIslandScope(ref: string, scope: IslandScope, selected: IslandBot | null, bots: readonly IslandBot[]) {
  if (scope === 'all') return true
  if (!selected) return false
  if (scope === 'bot') return ref === selected.ref
  const candidate = bots.find((bot) => bot.ref === ref)
  return !!selected.project && candidate?.project === selected.project && ref.split('/')[0] === selected.ref.split('/')[0]
}

const searchable = (value: string) => value.normalize('NFD').replace(/\p{M}/gu, '').toLocaleLowerCase()
export function searchIslandBots(bots: readonly IslandBot[], query: string) {
  const terms = searchable(query.trim()).split(/\s+/).filter(Boolean)
  return bots.filter((bot) => {
    const text = searchable(`${bot.name} ${bot.folder} ${bot.project ?? ''} ${bot.backend} ${bot.status}`)
    return terms.every((term) => text.includes(term))
  })
}

export function elapsedIslandRun(start: number | null | undefined, end: number | null | undefined, now: number) {
  if (!Number.isFinite(start) || !start || start < 0) return null
  const duration = (end ?? now) - start
  return Number.isFinite(duration) && duration >= 0 ? Math.floor(duration / 1000) : null
}

export interface IslandHistoryItem { key: string; text: string; ref: string | null; count: number; firstAt: number; at: number }
export interface IslandHistory { items: IslandHistoryItem[]; active: string[] }

export function normalizeIslandHistory(raw: unknown): IslandHistory {
  if (!raw || typeof raw !== 'object') return { items: [], active: [] }
  const value = raw as Partial<IslandHistory>
  return {
    items: Array.isArray(value.items) ? value.items.filter((item) => item && typeof item.key === 'string' && typeof item.text === 'string' && (item.ref === null || typeof item.ref === 'string') && Number.isFinite(item.at) && Number.isFinite(item.firstAt) && Number.isSafeInteger(item.count) && item.count > 0).slice(0, 50).map((item) => ({ ...item, text: item.text.slice(0, 500), key: item.key.slice(0, 600) })) : [],
    active: Array.isArray(value.active) ? value.active.filter((key): key is string => typeof key === 'string').slice(0, 200) : [],
  }
}

/** Count a notice on arrival/reoccurrence, never on each unchanged summary poll. */
export function updateIslandHistory(previous: IslandHistory, notices: { key: string; text: string; ref?: string }[], now: number): IslandHistory {
  const items = previous.items.map((item) => ({ ...item }))
  const unique = [...new Map(notices.map((notice) => [notice.key, notice])).values()].slice(0, 200)
  for (const notice of unique) {
    if (previous.active.includes(notice.key)) continue
    const existing = items.find((item) => item.key === notice.key)
    if (existing) { existing.count++; existing.at = now }
    else items.push({ ...notice, ref: notice.ref ?? null, count: 1, firstAt: now, at: now })
  }
  return { items: items.sort((a, b) => b.at - a.at).slice(0, 50), active: unique.map((notice) => notice.key) }
}
