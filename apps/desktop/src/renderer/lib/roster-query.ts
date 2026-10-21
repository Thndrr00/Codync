import type { Bot } from '../../shared/models.ts'

export type RosterFilter = 'all' | 'unread' | 'attention' | 'working' | 'pinned'
export type RosterGrouping = 'manual' | 'project'
export const rosterFilters: { id: RosterFilter; label: string }[] = [
  { id: 'all', label: 'All' }, { id: 'unread', label: 'Unread' },
  { id: 'attention', label: 'Needs attention' }, { id: 'working', label: 'Working' }, { id: 'pinned', label: 'Pinned' },
]

export function matchesRoster(bot: Bot, query: string, filter: RosterFilter): boolean {
  const matches = filter === 'all' || (filter === 'unread' && bot.unread > 0)
    || (filter === 'attention' && ['needsInput', 'error'].includes(bot.status))
    || (filter === 'working' && bot.status === 'working') || (filter === 'pinned' && bot.pinned)
  return matches && query.toLocaleLowerCase().trim().split(/\s+/).every((term) =>
    [bot.name, bot.description, bot.cwd, bot.backend].some((value) => value.toLocaleLowerCase().includes(term)))
}

export function rosterProject(bot: Bot): string {
  if (bot.kind === 'group') return 'Group chats'
  return bot.cwd || 'No project'
}

export function fastLabel(bot: Bot): string | null {
  if (bot.kind === 'group' || !bot.fastMode) return null
  switch (bot.fastModeSupport) {
    case 'offered': return 'Fast'
    case 'refused': return 'Fast refused'
    case 'unsupported': return 'Fast unavailable'
    default: return 'Fast requested'
  }
}

export function rosterElapsed(start: number | null | undefined, now: number): string {
  if (!start || start > now) return 'Duration unavailable'
  const seconds = Math.floor((now - start) / 1000)
  const minutes = Math.floor(seconds / 60)
  return minutes < 60 ? `${minutes}:${String(seconds % 60).padStart(2, '0')}`
    : `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`
}
