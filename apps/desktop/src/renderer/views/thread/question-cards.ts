import type { Entry } from '@shared/models'
import { questionCardsAfter, type QuestionCardRef } from '../../lib/question-overlap.ts'

export interface OpenQuestion extends QuestionCardRef {
  entry: Entry
}

const chatKey = (botId: string, threadId: string | null) => `${botId}/${threadId ?? ''}`

/**
 * The question cards still visible in one chat, oldest first. Every card is folded through the
 * pure overlap decision, so a routine question posting or being answered never closes the agent's
 * earlier question here either (OpenBot nightly triage-15338). Kept out of the view so it can be
 * tested without React or a store.
 */
export function openQuestionCards(entries: readonly Entry[], botId: string, threadId: string | null): OpenQuestion[] {
  let cards: OpenQuestion[] = []
  for (const entry of entries) {
    if (entry.kind !== 'permission' || entry.data.toolKind !== 'question') continue
    if (entry.botId !== botId || (entry.threadId ?? null) !== threadId) continue
    const status = entry.data.status ?? 'pending'
    const incoming: OpenQuestion = {
      id: entry.id,
      chat: chatKey(botId, threadId),
      open: status === 'pending' || status === 'expired',
      entry,
    }
    cards = questionCardsAfter(cards, incoming)
  }
  return cards
}
