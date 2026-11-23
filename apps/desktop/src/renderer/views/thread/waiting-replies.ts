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
  return rows
    .filter((row) => row.state !== 'failed' || row.entry.turn >= latestUserTurn)
    .sort((a, b) => a.id.localeCompare(b.id))
}
