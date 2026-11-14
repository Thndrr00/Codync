import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { Entry } from '../../../shared/models.ts'
import { buildChat } from './chat-items.ts'

const at = 1_760_000_000_000
let seq = 0

/** A message said at the computer (`person: null`) or from the device `person`. */
function said(person: string | null, kind = 'user'): Entry {
  seq += 1
  return {
    id: `e${seq}`,
    seq,
    botId: 'b',
    rev: 1,
    kind,
    turn: 0,
    data: person ? { text: 'hi', person: { key: person, name: person, color: 'blue', typing: false } } : { text: 'hi' },
    createdAt: at + seq * 1000,
    updatedAt: at + seq * 1000,
  }
}

const starts = (entries: Entry[]) =>
  buildChat(entries)
    .filter((item) => item.kind === 'entry')
    .map((item) => (item.kind === 'entry' ? item.groupStart : false))

test('each device starts its own run of messages, and repeats do not', () => {
  assert.deepEqual(starts([said('ana'), said('ana'), said('bo'), said(null), said(null)]), [true, false, true, true, false])
})

test('two devices do not share a run', () => {
  assert.deepEqual(starts([said('bo'), said('ana')]), [true, true])
})
