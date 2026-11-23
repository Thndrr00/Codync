import assert from 'node:assert/strict'
import { test } from 'node:test'
import { subagentRows, type SubagentSource } from './bot-subagents.ts'

const ask = (id: string, peerId: string, outcome: SubagentSource['outcome']): SubagentSource =>
  ({ id, peerId, outcome, request: `Task ${id}` })

test('one row per teammate, in first-ask order, with the worst status', () => {
  const rows = subagentRows([
    ask('a', 'owen', 'done'),
    ask('b', 'egan', 'pending'),
    ask('c', 'owen', 'failed'),
  ])
  assert.deepEqual(rows.map((r) => [r.id, r.peerId, r.status, r.count]), [
    ['a', 'owen', 'failed', 2],
    ['b', 'egan', 'running', 1],
  ])
})

test('a completed ask after a running one stays running', () => {
  assert.equal(subagentRows([ask('a', 'owen', 'pending'), ask('b', 'owen', 'done')])[0]!.status, 'running')
})

test('no asks make no card', () => {
  assert.deepEqual(subagentRows([]), [])
})
