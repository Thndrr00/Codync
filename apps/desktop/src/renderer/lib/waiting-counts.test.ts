import assert from 'node:assert/strict'
import { test } from 'node:test'
import { waitingCounts, type WaitState } from './waiting-counts.ts'

const states = (...values: WaitState[]) => values.map((state) => ({ state }))

test('all requests failed: ended with a failed count and no waiting', () => {
  assert.deepEqual(waitingCounts(states('failed', 'failed')), {
    total: 2, asked: 0, stopped: 0, replied: 0, failed: 2, ended: true,
  })
})

test('a reply and a failure end the list with both counts', () => {
  assert.deepEqual(waitingCounts(states('replied', 'failed')), {
    total: 2, asked: 0, stopped: 0, replied: 1, failed: 1, ended: true,
  })
})

test('a request still asked keeps the waiting header', () => {
  assert.equal(waitingCounts(states('asked', 'failed', 'replied')).ended, false)
})

test('a stopped row keeps the waiting header, even beside a failure', () => {
  assert.equal(waitingCounts(states('stopped', 'failed')).ended, false)
})

test('all replies and an empty list are not an ended-with-failure list', () => {
  assert.equal(waitingCounts(states('replied', 'replied')).ended, false)
  assert.deepEqual(waitingCounts(states()), { total: 0, asked: 0, stopped: 0, replied: 0, failed: 0, ended: false })
})
