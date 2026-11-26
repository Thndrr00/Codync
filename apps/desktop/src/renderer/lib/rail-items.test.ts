import test from 'node:test'
import assert from 'node:assert/strict'
import { railItems } from './rail-items.ts'

test('right-click hides only the rail pages that can be hidden', () => {
  assert.deepEqual(railItems([]), ['marketplace', 'help', 'credits'])
  assert.deepEqual(railItems(['marketplace']), ['help', 'credits'])
  assert.deepEqual(railItems(['marketplace', 'help']), ['credits'])
})

test('Credits cannot be hidden and unknown ids change nothing', () => {
  assert.deepEqual(railItems(['credits']), ['marketplace', 'help', 'credits'])
  assert.deepEqual(railItems(['credits', 'a-stale-page']), ['marketplace', 'help', 'credits'])
  assert.deepEqual(railItems(null), ['marketplace', 'help', 'credits'])
})
