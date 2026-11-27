import assert from 'node:assert/strict'
import { test } from 'node:test'
import { unreadAction, unreadEntryId } from './roster-unread.ts'

const entry = (id: string, kind: string, final = true) => ({ id, kind, data: { final } })

test('there is no unread action without a message to start from', () => {
  assert.equal(unreadAction(null), null)
  assert.deepEqual(unreadAction('e3'), { entryId: 'e3' })
})

test('the newest final agent reply is what the roster marks unread', () => {
  const entries = [entry('u1', 'user'), entry('a1', 'agent'), entry('t1', 'thought'), entry('a2', 'agent')]
  assert.equal(unreadEntryId(entries), 'a2')
  assert.equal(unreadEntryId([entry('u1', 'user'), entry('a1', 'agent', false)]), null, 'a streaming reply is not chat-visible')
  assert.equal(unreadEntryId([]), null)
})
