import type { Entry } from '@shared/models'
import { botExchange, isOutgoing, peerOf, type BotExchange } from './bot-exchange.ts'

export interface WaitingReply {
  id: string
  /** The teammate that owes the answer. */
  name: string
  /** What the agent asked it. */
  request: string
  state: 'asked' | 'stopped'
  /** The notice that carries the ask, for the Stop action. */
  entry: Entry
}

/**
 * The "Asked" requests still waiting in this chat or thread, from the structured bot messages
 * the host publishes. A stopped row stays visible with its new state instead of vanishing, so
 * the person can see what they cancelled. Pure, so the shape can be tested without the store.
 */
export function waitingReplyRows(
  entries: Entry[],
  botId: string,
  threadId: string | null,
  stopped: ReadonlySet<string>,
  authorName: (id: string) => string,
): WaitingReply[] {
  const rows: WaitingReply[] = []
  for (const entry of entries) {
    const exchange: BotExchange | null = botExchange(entry)
    if (!exchange || !isOutgoing(exchange) || exchange.chat !== botId) continue
    if ((entry.threadId ?? null) !== threadId) continue
    const isStopped = stopped.has(entry.id)
    const asked = (entry.data.heading ?? '').startsWith('Asked')
    if (!isStopped && (!asked || exchange.outcome.kind !== 'pending')) continue
    rows.push({
      id: entry.id,
      name: exchange.message.peerName ?? authorName(peerOf(exchange)),
      request: exchange.message.text,
      state: isStopped ? 'stopped' : 'asked',
      entry,
    })
  }
  return rows.sort((a, b) => a.id.localeCompare(b.id))
}
