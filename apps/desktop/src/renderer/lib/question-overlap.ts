/**
 * The parts of a question card the overlap decision needs (OpenBot nightly triage-15338): its own
 * id, the chat it was posted in, and whether it is still open. A routine run's question and the
 * agent's earlier question are two cards, even when the person sees them in the same chat.
 */
export interface QuestionCardRef {
  id: string
  chat: string
  open: boolean
}

/**
 * Whether the posting or answer of `incoming` may close `existing`. Each card owns its own
 * lifecycle: only the card the event names may close, so a routine question never hides the
 * agent's earlier one in the same chat, and answering either leaves the other answerable. A card
 * in another chat is a different card even if an id happens to match. Pure, tested.
 */
export function questionCanClose(existing: QuestionCardRef, incoming: QuestionCardRef): boolean {
  return existing.id === incoming.id && existing.chat === incoming.chat
}

/**
 * The visible question-card list after a card is posted or answered. `incoming` replaces only
 * itself; a different card — in the same chat or another — keeps its place and its own status.
 * This is the client half of the host's card list, so the two never disagree about which cards
 * are still waiting. Pure, tested.
 */
export function questionCardsAfter<T extends QuestionCardRef>(cards: readonly T[], incoming: T): T[] {
  const kept = cards.filter((card) => !questionCanClose(card, incoming))
  return [...kept, incoming]
}
