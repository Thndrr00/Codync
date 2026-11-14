import type { CustomIcon } from './custom-icons'
import type { IslandBot } from './island'

// The island's agent dock: a compact roster of the bots the chat window already sends the island,
// shown on its own edge and sizing. Off by default (`islandAgentDock`); it never gets a window of
// its own, so multi-display bounds, shaping and pointer handling stay the island's.

/** What the dock says a bot is doing. A status newer than this app counts as idle. */
export type AgentDockState = 'working' | 'needsInput' | 'error' | 'idle'

/** One agent in the dock, in the roster's order. */
export interface AgentDockRow {
  ref: string
  name: string
  shape: string
  color: string
  customIcon?: CustomIcon
  state: AgentDockState
  /** Its one line: what it's doing, what it needs, why it stopped, or its last message. */
  activity: string
  unread: number
}

/** The collapsed island's line: a phrase with a `{count}` slot, and the count that fills it. */
export interface AgentDockLine {
  key: string
  count: number | null
}

export function agentDockState(status: string): AgentDockState {
  switch (status) {
    case 'working':
      return 'working'
    case 'needsInput':
      return 'needsInput'
    case 'error':
      return 'error'
    default:
      return 'idle'
  }
}

/** The dock's rows, in the order the chat window sent them (the app's roster, groups aside). */
export function agentDockRows(bots: IslandBot[]): AgentDockRow[] {
  return bots.map((bot) => ({
    ref: bot.ref,
    name: bot.name,
    shape: bot.shape,
    color: bot.color,
    customIcon: bot.customIcon,
    state: agentDockState(bot.status),
    activity: bot.error ?? bot.detail,
    unread: typeof bot.unread === 'number' && Number.isFinite(bot.unread) ? Math.max(0, Math.floor(bot.unread)) : 0,
  }))
}

/** How many of the dock's rows are in each state. */
export function agentDockCounts(rows: AgentDockRow[]) {
  const counts = { working: 0, needsYou: 0, failed: 0, idle: 0 }
  for (const row of rows) {
    if (row.state === 'working') counts.working += 1
    else if (row.state === 'needsInput') counts.needsYou += 1
    else if (row.state === 'error') counts.failed += 1
    else counts.idle += 1
  }
  return counts
}

/** What the dock, or the collapsed island, says about them: needs you, working, stopped, idle. */
export function agentDockLine(rows: AgentDockRow[]): AgentDockLine {
  const { working, needsYou, failed } = agentDockCounts(rows)
  if (needsYou === 1) return { key: 'Needs you', count: null }
  if (needsYou > 1) return { key: '{count} need you', count: needsYou }
  if (working === 1) return { key: '1 agent working', count: null }
  if (working > 1) return { key: '{count} agents working', count: working }
  if (failed === 1) return { key: '1 agent stopped', count: null }
  if (failed > 1) return { key: '{count} agents stopped', count: failed }
  if (rows.length === 0) return { key: 'No agents on this computer', count: null }
  return { key: 'All agents idle', count: null }
}

/** The line for display. The view passes its translator, so a translated phrase still gets the count. */
export function agentDockText(line: AgentDockLine, translate: (text: string) => string = (text) => text): string {
  const phrase = translate(line.key)
  return line.count === null ? phrase : phrase.replaceAll('{count}', () => String(line.count))
}
