import assert from 'node:assert/strict'
import { test } from 'node:test'
import { ownSetupTerminal, revealOwnedSetupTerminal } from './setup-terminal-owners.ts'

test('terminal owners are scoped to the exact store and session and detach without controlling a process', () => {
  const store = {}, other = {}
  let focused = 0
  const release = ownSetupTerminal(store, 'session', () => focused++)
  assert.equal(revealOwnedSetupTerminal(other, 'session'), false)
  assert.equal(revealOwnedSetupTerminal(store, 'other'), false)
  assert.equal(revealOwnedSetupTerminal(store, 'session'), true)
  assert.equal(focused, 1)
  release()
  assert.equal(revealOwnedSetupTerminal(store, 'session'), false)
})

test('an old owner cleanup cannot remove its replacement', () => {
  const store = {}
  const old = ownSetupTerminal(store, 'session', () => assert.fail('stale owner'))
  let focused = 0
  const current = ownSetupTerminal(store, 'session', () => focused++)
  old()
  assert.equal(revealOwnedSetupTerminal(store, 'session'), true)
  assert.equal(focused, 1)
  current()
})
