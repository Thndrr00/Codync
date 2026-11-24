import assert from 'node:assert/strict'
import { test } from 'node:test'
import { questionCanClose, questionCardsAfter, type QuestionCardRef } from './question-overlap.ts'

const card = (id: string, chat: string, open = true): QuestionCardRef => ({ id, chat, open })
const ids = (cards: readonly QuestionCardRef[]) => cards.map((c) => c.id)

test('two overlapping questions in one chat each keep their own lifecycle', () => {
  const agent = card('agent-q', 'chat')
  const routine = card('routine-q', 'chat')
  assert.equal(questionCanClose(agent, routine), false, 'a routine question cannot close the agent\'s')
  assert.equal(questionCanClose(routine, agent), false)
  assert.equal(questionCanClose(agent, agent), true, 'a card may still close itself')
  assert.deepEqual(ids(questionCardsAfter([agent], routine)), ['agent-q', 'routine-q'])
})

test('answering the routine question first leaves the agent question visible and answerable', () => {
  const agent = card('agent-q', 'chat')
  const routine = card('routine-q', 'chat')
  const answered = questionCardsAfter([agent, routine], card('routine-q', 'chat', false))
  assert.deepEqual(ids(answered), ['agent-q', 'routine-q'])
  assert.deepEqual(answered.find((c) => c.id === 'agent-q'), agent)
  assert.equal(questionCanClose(agent, card('routine-q', 'chat', false)), false)
  // A repeat answer replaces the named card with the same closed status: no second commit.
  const repeat = questionCardsAfter(answered, card('routine-q', 'chat', false))
  assert.deepEqual(repeat, answered)
})

test('answering the agent question first leaves the routine question visible', () => {
  const agent = card('agent-q', 'chat')
  const routine = card('routine-q', 'chat')
  const answered = questionCardsAfter([agent, routine], card('agent-q', 'chat', false))
  assert.deepEqual(ids(answered), ['routine-q', 'agent-q'])
  assert.deepEqual(answered.find((c) => c.id === 'routine-q'), routine)
})

test('a card in another chat is never closed by a post or an answer', () => {
  const here = card('q', 'chat-a')
  const there = card('q', 'chat-b')
  assert.equal(questionCanClose(here, there), false)
  assert.equal(questionCanClose(there, here), false)
  assert.deepEqual(ids(questionCardsAfter([here], there)), ['q', 'q'])
})

test('an expired routine question still cannot hide the earlier agent question', () => {
  const agent = card('agent-q', 'chat')
  const expired = card('routine-q', 'chat', true) // expired is still open and answerable
  assert.equal(questionCanClose(agent, expired), false)
  assert.deepEqual(ids(questionCardsAfter([agent], expired)), ['agent-q', 'routine-q'])
  const closed = card('routine-q', 'chat', false)
  assert.equal(questionCanClose(agent, closed), false, 'a closed routine card still cannot close the agent card')
})
