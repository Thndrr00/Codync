// The status card a thread draws for the subagents its agent started (OpenBot nightly triage-13639).
// One row per teammate, worst status wins, oldest ask first. Pure: the view only maps host entries
// into these sources and renders the result.

export type SubagentStatus = 'running' | 'completed' | 'failed'

export interface SubagentSource {
  /** The notice that carried the ask. */
  id: string
  peerId: string
  peerName?: string
  outcome: 'pending' | 'done' | 'failed'
  request: string
}

export interface SubagentRow {
  id: string
  peerId: string
  peerName?: string
  status: SubagentStatus
  request: string
  /** How many asks went to this teammate. */
  count: number
}

const rank: Record<SubagentStatus, number> = { completed: 0, running: 1, failed: 2 }
const statusOf = (outcome: SubagentSource['outcome']): SubagentStatus =>
  outcome === 'failed' ? 'failed' : outcome === 'pending' ? 'running' : 'completed'

export function subagentRows(sources: readonly SubagentSource[]): SubagentRow[] {
  const rows: SubagentRow[] = []
  const byPeer = new Map<string, SubagentRow>()
  for (const source of sources) {
    const known = byPeer.get(source.peerId)
    if (known) {
      known.count += 1
      const status = statusOf(source.outcome)
      if (rank[status] > rank[known.status]) known.status = status
      continue
    }
    const row: SubagentRow = {
      id: source.id,
      peerId: source.peerId,
      peerName: source.peerName,
      status: statusOf(source.outcome),
      request: source.request,
      count: 1,
    }
    byPeer.set(source.peerId, row)
    rows.push(row)
  }
  return rows
}
