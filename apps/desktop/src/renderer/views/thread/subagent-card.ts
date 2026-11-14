import type { Entry, Subagent } from '@shared/models'

/** The state of one subagent, as its row draws it: a host older than the `subagents` card never sends one. */
export function subagentState(subagent: Subagent): 'running' | 'done' | 'failed' | 'stopped' {
  return subagent.state === 'done' || subagent.state === 'failed' || subagent.state === 'stopped'
    ? subagent.state
    : 'running'
}

/** Whether a card still has a subagent at work: only then does its header follow the reader. */
export function runningSubagents(entry: Entry): boolean {
  return entry.kind === 'subagents' && (entry.data.subagents ?? []).some((s) => subagentState(s) === 'running')
}

/** The newest turn with a subagents card whose work still runs: the one the dock follows. */
export function activeSubagentCard(entries: Entry[]): Entry | undefined {
  for (let i = entries.length - 1; i >= 0; i -= 1) {
    const entry = entries[i]!
    if (runningSubagents(entry)) return entry
  }
  return undefined
}

/** "12s" or "1m 05s": how long a subagent, or all of them, took. */
export function subagentDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1_000))
  if (total < 60) return `${total}s`
  return `${Math.floor(total / 60)}m ${String(total % 60).padStart(2, '0')}s`
}
