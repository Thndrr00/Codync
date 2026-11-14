/**
 * A chat's pinned messages: a decision, a path or a detail that must stay at hand. A pin rides on
 * the message (`EntryData.pinned`), so a pin or unpin reaches every client as that message's new rev.
 */

/** The most pins one chat keeps; the host refuses one over it. */
export const PINS_LIMIT = 50

/** Who pinned a message: a person (their device, or none at the computer itself) or the chat's agent. */
export type Pinner = { kind: 'person'; device: string | null } | { kind: 'agent'; botId: string }

/** A pinned message of a chat or one of its threads. */
export interface PinnedMessage {
  messageId: string
  /** The side thread the message is in, or null for the chat itself. */
  threadId: string | null
  threadTitle: string | null
  pinnedAt: number
  pinnedBy: Pinner
  /** Who wrote it: a person, or this chat's agent. */
  author: 'user' | 'agent'
  /** The other bot that wrote it, in a group; null otherwise. */
  authorBotId: string | null
  createdAt: number
  /** The start of the message, or the names of its files when it has no text. */
  text: string
}
