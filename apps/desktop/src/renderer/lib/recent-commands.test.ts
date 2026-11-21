import { test } from 'node:test'
import assert from 'node:assert/strict'
import { pushRecentCommand } from './recent-commands.ts'

test('a repeat pick moves to the front without appearing twice', () => {
  assert.deepEqual(pushRecentCommand(['settings', 'send', 'search'], 'send'), ['send', 'settings', 'search'])
})

test('a new pick is remembered and the list stays capped', () => {
  assert.deepEqual(pushRecentCommand(['a', 'b', 'c'], 'd', 3), ['d', 'a', 'b'])
  assert.deepEqual(pushRecentCommand(Array.from({ length: 9 }, (_, i) => `c${i}`), 'new'), [
    'new', 'c0', 'c1', 'c2', 'c3', 'c4',
  ])
})

test('an empty pick leaves the recent list untouched', () => {
  assert.deepEqual(pushRecentCommand(['a'], '  '), ['a'])
})
