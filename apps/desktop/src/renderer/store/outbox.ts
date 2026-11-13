// Messages typed while a computer is offline wait here, on this device, and go out when its host is
// back. One list for every computer; each entry keeps the nonce that makes a repeat send harmless.

import type { ReplyQuote } from '@shared/models'

export interface QueuedMessage {
  computerId: string
  botId: string
  text: string
  nonce: string
  pasted: string[]
  thread: string | null
  replyTo: ReplyQuote | null
  at: number
}

const KEY = 'codync:outbox'

export function queuedMessages(): QueuedMessage[] {
  try {
    const raw = localStorage.getItem(KEY)
    const parsed: unknown = raw ? JSON.parse(raw) : []
    return Array.isArray(parsed) ? (parsed as QueuedMessage[]) : []
  } catch {
    return []
  }
}

function save(messages: QueuedMessage[]) {
  try {
    if (messages.length) localStorage.setItem(KEY, JSON.stringify(messages))
    else localStorage.removeItem(KEY)
  } catch {
    // No storage: the message stays only in this window's optimistic row.
  }
}

export function queueMessage(message: QueuedMessage) {
  save([...queuedMessages().filter((item) => item.nonce !== message.nonce), message])
}

export function dropQueued(nonce: string) {
  save(queuedMessages().filter((item) => item.nonce !== nonce))
}

/** Removes and returns this computer's queued messages, oldest first. */
export function takeQueued(computerId: string): QueuedMessage[] {
  const all = queuedMessages()
  const mine = all.filter((item) => item.computerId === computerId).sort((a, b) => a.at - b.at)
  save(all.filter((item) => item.computerId !== computerId))
  return mine
}
