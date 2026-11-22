import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, EntryData } from '../../../shared/models.ts'
import { waitingReplyRows } from './waiting-replies.ts'

const asked = (id: string, status: string, extra: Partial<Entry> = {}): Entry => {
  const data: EntryData = {
    text: `Asked Owen: ${id}`,
    heading: `Asked Owen: ${id}`,
    status,
    botMessage: { sourceBotId: 'egan', targetBotId: 'owen', text: `Inspect ${id}` },
  }
  return { id, seq: 1, botId: 'egan', rev: 1, kind: 'notice', turn: 0, createdAt: 1, updatedAt: 1, data, ...extra }
}

test('only pending Asked rows wait for a reply, and a stopped row keeps its place', () => {
  const waiting = asked('waiting', 'queued')
  const done = asked('done', 'completed')
  const stopped = asked('stopped', 'cancelled')
  const rows = waitingReplyRows([waiting, done, stopped], 'egan', null, new Set(['stopped']), (id) => `Name:${id}`)
  assert.deepEqual(rows.map(({ id, name, request, state }) => ({ id, name, request, state })), [
    { id: 'stopped', name: 'Name:owen', request: 'Inspect stopped', state: 'stopped' },
    { id: 'waiting', name: 'Name:owen', request: 'Inspect waiting', state: 'asked' },
  ])
  assert.equal(rows[0]!.entry.id, 'stopped')
})

test('a message that was not an ask, a replied ask, or a row from another thread do not wait', () => {
  const message: Entry = { ...asked('message', 'queued'), data: {
    text: 'Messaged Owen: hi', heading: 'Messaged Owen: hi',
    botMessage: { sourceBotId: 'egan', targetBotId: 'owen', text: 'hi' },
  } }
  const rows = waitingReplyRows(
    [message, asked('done', 'completed'), asked('thread', 'queued', { threadId: 'other' }), asked('mine', 'queued')],
    'egan', null, new Set(), (id) => id,
  )
  assert.deepEqual(rows.map((row) => row.id), ['mine'])
})
