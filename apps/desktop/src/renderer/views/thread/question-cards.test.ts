import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, EntryData } from '../../../shared/models.ts'
import { openQuestionCards } from './question-cards.ts'

const question = (id: string, status: string, extra: Partial<Entry> = {}): Entry => {
  const data: EntryData = { toolKind: 'question', title: `Question ${id}`, status }
  return {
    id, seq: 1, botId: 'bot', rev: 1, kind: 'permission', turn: 1,
    createdAt: 1, updatedAt: 1, data, ...extra,
  }
}

test('both questions stay in the chat card list, and answering one keeps the other', () => {
  const agent = question('agent-q', 'pending')
  const routine = question('routine-q', 'pending')
  assert.deepEqual(openQuestionCards([agent, routine], 'bot', null).map((c) => c.id), ['agent-q', 'routine-q'])
  const answered = openQuestionCards([agent, question('routine-q', 'answered')], 'bot', null)
  assert.deepEqual(answered.map((c) => [c.id, c.open]), [['agent-q', true], ['routine-q', false]])
})

test('an expired routine question is still visible and never hides the agent question', () => {
  const cards = openQuestionCards([question('agent-q', 'pending'), question('routine-q', 'expired')], 'bot', null)
  assert.deepEqual(cards.map((c) => [c.id, c.open]), [['agent-q', true], ['routine-q', true]])
})

test('cards in another chat or thread are left out', () => {
  const cards = openQuestionCards([
    question('agent-q', 'pending'),
    question('other-bot', 'pending', { botId: 'other' }),
    question('other-thread', 'pending', { threadId: 'thread' }),
  ], 'bot', null)
  assert.deepEqual(cards.map((c) => c.id), ['agent-q'])
})
