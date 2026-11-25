import assert from 'node:assert/strict'
import { test } from 'node:test'
import { splashDelay, splashVisible, SPLASH_MIN_MS, workspaceVisible } from './splash-visible.ts'

test('the splash stays up for its minimum even when the renderer is ready at once', () => {
  assert.equal(SPLASH_MIN_MS, 1500)
  assert.equal(splashVisible(0, true), true)
  assert.equal(splashVisible(1499, true), true)
  assert.equal(splashVisible(1500, true), false)
  assert.equal(workspaceVisible(1500, true), true)
})

test('a slow renderer shows the workspace as soon as it is ready', () => {
  assert.equal(splashVisible(3200, true), false)
  assert.equal(workspaceVisible(3200, true), true)
  assert.equal(splashDelay(3200, true), 0)
})

test('an unready or broken clock keeps the splash up with a bounded wait', () => {
  assert.equal(splashVisible(5000, false), true)
  assert.equal(workspaceVisible(5000, false), false)
  assert.equal(splashDelay(5000, false), SPLASH_MIN_MS)
  assert.equal(splashVisible(NaN, true), true)
  assert.equal(splashDelay(Infinity, true), SPLASH_MIN_MS)
})

test('the minimum is configurable for a faster UI check', () => {
  assert.equal(splashVisible(80, true, 100), true)
  assert.equal(splashDelay(80, true, 100), 20)
  assert.equal(workspaceVisible(100, true, 100), true)
})
