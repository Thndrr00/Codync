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

test('only asks wait, and a stopped row keeps its place', () => {
  const waiting = asked('waiting', 'queued')
  const stopped = asked('stopped', 'cancelled')
  const rows = waitingReplyRows([waiting, stopped], 'egan', null, new Set(['stopped']), (id) => `Name:${id}`)
  assert.deepEqual(rows.map(({ id, name, request, state }) => ({ id, name, request, state })), [
    { id: 'stopped', name: 'Name:owen', request: 'Inspect stopped', state: 'stopped' },
    { id: 'waiting', name: 'Name:owen', request: 'Inspect waiting', state: 'asked' },
  ])
  assert.equal(rows[0]!.entry.id, 'stopped')
})

test('a replied ask is counted and a failed ask is kept so the header can end', () => {
  const rows = waitingReplyRows([asked('done', 'completed'), asked('failed', 'failed')], 'egan', null, new Set(), (id) => id)
  assert.deepEqual(rows.map((row) => [row.id, row.state]), [['done', 'replied'], ['failed', 'failed']])
})

test('a failed ask is cleared by the next message the person sends', () => {
  const failed = asked('failed', 'failed')
  const nextMessage = { ...asked('next', 'queued'), kind: 'user', turn: 1, data: { text: 'Try again' } }
  const rows = waitingReplyRows([failed, nextMessage], 'egan', null, new Set(), (id) => id)
  assert.deepEqual(rows.map((row) => row.id), [])
})

test('a message that was not an ask, and a row from another thread, are not asks', () => {
  const message: Entry = { ...asked('message', 'queued'), data: {
    text: 'Messaged Owen: hi', heading: 'Messaged Owen: hi',
    botMessage: { sourceBotId: 'egan', targetBotId: 'owen', text: 'hi' },
  } }
  const rows = waitingReplyRows(
    [message, asked('thread', 'queued', { threadId: 'other' }), asked('mine', 'queued')],
    'egan', null, new Set(), (id) => id,
  )
  assert.deepEqual(rows.map((row) => row.id), ['mine'])
})
