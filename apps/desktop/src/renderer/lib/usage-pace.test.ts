import { test } from 'node:test'
import assert from 'node:assert/strict'
import { pacePercent } from './usage-pace.ts'

const HOUR = 3_600_000

test('the mark is halfway through a window halfway through its time', () => {
  assert.equal(pacePercent({ resetsAt: HOUR * 2, windowSeconds: 7200 }, HOUR), 50)
})

test('the mark sits at the start at the start, and at the end past the reset', () => {
  assert.equal(pacePercent({ resetsAt: HOUR, windowSeconds: 3600 }, 0), 0)
  assert.equal(pacePercent({ resetsAt: HOUR, windowSeconds: 3600 }, HOUR * 3), 100, 'never past the bar')
})

test('no reset or no length leaves no mark', () => {
  assert.equal(pacePercent({ windowSeconds: 3600 }, HOUR), null)
  assert.equal(pacePercent({ resetsAt: HOUR }, HOUR), null)
  assert.equal(pacePercent({ resetsAt: HOUR, windowSeconds: 0 }, HOUR), null)
})
