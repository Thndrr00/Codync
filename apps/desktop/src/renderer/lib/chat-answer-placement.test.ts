import assert from 'node:assert/strict'
import { test } from 'node:test'
import { newFinalAnswerAtEnd, type AnswerRow } from './chat-answer-placement.ts'

const row = (id: string, kind: AnswerRow['kind'] = 'entry', author = 'agent', final = true): AnswerRow => ({ id, kind, author, final })

test('a final answer appended at the end is placed at its start', () => {
  assert.equal(newFinalAnswerAtEnd(['a', 'b'], [row('a'), row('b'), row('reply')]), 'reply')
  assert.equal(newFinalAnswerAtEnd(['a'], [row('a'), row('reply')]), 'reply')
})

test('a reply that arrived with a time separator is still appended', () => {
  assert.equal(newFinalAnswerAtEnd(['a'], [row('a'), row('sep-a', 'separator', ''), row('reply')]), 'reply')
})

test('an opening chat, a seen answer and a short answer keep the old scroll', () => {
  assert.equal(newFinalAnswerAtEnd(null, [row('reply')]), null)
  assert.equal(newFinalAnswerAtEnd(['a', 'reply'], [row('a'), row('reply')]), null)
})

test('a sent message, a progress note and loaded history do not place the answer', () => {
  assert.equal(newFinalAnswerAtEnd(['a'], [row('a'), row('mine', 'entry', 'user', false)]), null)
  assert.equal(newFinalAnswerAtEnd(['a'], [row('a'), row('progress', 'entry', 'agent', false)]), null)
  assert.equal(newFinalAnswerAtEnd(['b', 'reply'], [row('a'), row('b'), row('reply')]), null, 'older page prepends, the last row is not new')
})

test('a bot-to-bot exchange is not a final answer', () => {
  assert.equal(newFinalAnswerAtEnd(['a'], [row('a'), row('asked', 'exchanges', '', false)]), null)
})
