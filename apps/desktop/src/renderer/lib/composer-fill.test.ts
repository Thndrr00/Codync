import assert from 'node:assert/strict'
import { test } from 'node:test'
import { composer_fill } from './composer-fill.ts'

test('the composer fill appears only with both scrolling threads and a transparent window', () => {
  assert.equal(composer_fill(true, true), true)
})

test('a solid window never gets a fill, even with threads scrolling under the composer', () => {
  assert.equal(composer_fill(true, false), false)
})

test('threads that do not scroll under the composer never get a fill', () => {
  assert.equal(composer_fill(false, true), false)
  assert.equal(composer_fill(false, false), false)
})
