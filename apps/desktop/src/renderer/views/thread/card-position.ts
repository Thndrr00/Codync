import type { Entry } from '@shared/models'

// A card posted during a turn stays where it first appeared, even after the turn answers it and
// later messages arrive (OpenBot 0.78.0). Pure: no React, no store, no scroll behavior here.

/** A card anchored to the entry it was posted from. */
export interface AnchoredCard {
  id: string
}

/**
 * Put each card at the position of its original entry. Folding an answer into a card list appends
 * the updated card at the end, which otherwise drags a mid-turn card below later messages. Entries
 * are the row order's authority; a card with no entry keeps its incoming order after the anchored
 * ones. Stable for equal positions. Tested without a DOM.
 */
export function card_position<T extends AnchoredCard>(cards: readonly T[], entries: readonly Entry[]): T[] {
  if (cards.length < 2) return [...cards]
  const position = new Map<string, number>()
  entries.forEach((entry, index) => {
    if (!position.has(entry.id)) position.set(entry.id, index)
  })
  const afterAll = entries.length + cards.length
  const rank = (card: T) => position.get(card.id) ?? afterAll
  return [...cards].sort((a, b) => rank(a) - rank(b))
}
