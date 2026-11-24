import type { Entry } from '@shared/models'
import type { WaitState } from '../../lib/waiting-counts.ts'
import { botExchange, isOutgoing, peerOf, type BotExchange } from './bot-exchange.ts'

export interface WaitingReply {
  id: string
  /** The teammate that owes the answer. */
  name: string
  /** What the agent asked it. */
  request: string
  state: WaitState
  /** The notice that carries the ask, for the Stop action. */
  entry: Entry
}

/**
 * OpenBot 0.77: asking the same teammate again retires the older failed "Asked" row, even when the
 * person sent no new message in between. Only a *newer* ask to that same peer clears it: a failure
 * to someone else, or a re-ask that came first, leaves the row standing. Pure, tested.
 */
export function supersededFailedAsks(rows: WaitingReply[]): Set<string> {
  const latest = new Map<string, number>()
  for (const row of rows) {
    const exchange = botExchange(row.entry)
    if (!exchange) continue
    const peer = peerOf(exchange)
    latest.set(peer, Math.max(latest.get(peer) ?? Number.NEGATIVE_INFINITY, row.entry.seq))
  }
  const cleared = new Set<string>()
  for (const row of rows) {
    if (row.state !== 'failed') continue
    const exchange = botExchange(row.entry)
    if (!exchange) continue
    const last = latest.get(peerOf(exchange))
    if (last !== undefined && last > row.entry.seq) cleared.add(row.id)
  }
  return cleared
}

/**
 * The asks in this chat or thread, from the structured bot messages the host publishes. A stopped
 * row keeps its place, and a row that replied or failed stays long enough to count: the card hides
 * replied rows and keeps failed ones until it is closed (OpenBot nightly triage-13388). Pure, so
 * the shape can be tested without the store.
 */
export function waitingReplyRows(
  entries: Entry[],
  botId: string,
  threadId: string | null,
  stopped: ReadonlySet<string>,
  authorName: (id: string) => string,
): WaitingReply[] {
  const rows: WaitingReply[] = []
  // A failed row stays through the turn that caused it; the next thing the person says clears it.
  let latestUserTurn = Number.NEGATIVE_INFINITY
  for (const entry of entries) {
    if (entry.kind === 'user' && (entry.threadId ?? null) === threadId) latestUserTurn = Math.max(latestUserTurn, entry.turn)
  }
  for (const entry of entries) {
    const exchange: BotExchange | null = botExchange(entry)
    if (!exchange || !isOutgoing(exchange) || exchange.chat !== botId) continue
    if ((entry.threadId ?? null) !== threadId) continue
    const isStopped = stopped.has(entry.id)
    const asked = (entry.data.heading ?? '').startsWith('Asked')
    if (!asked && !isStopped) continue
    const state: WaitState = isStopped ? 'stopped'
      : exchange.outcome.kind === 'failed' ? 'failed'
        : exchange.outcome.kind === 'done' ? 'replied'
          : 'asked'
    rows.push({
      id: entry.id,
      name: exchange.message.peerName ?? authorName(peerOf(exchange)),
      request: exchange.message.text,
      state,
      entry,
    })
  }
  const superseded = supersededFailedAsks(rows)
  return rows
    .filter((row) => row.state !== 'failed' || (row.entry.turn >= latestUserTurn && !superseded.has(row.id)))
    .sort((a, b) => a.id.localeCompare(b.id))
}
