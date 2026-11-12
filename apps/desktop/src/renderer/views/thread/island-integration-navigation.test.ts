import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Backend } from '../../../shared/models.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'

function clearPending() {
  const pending = islandThreadNavigation.pending
  if (pending) islandThreadNavigation.consume(pending.ref)
}

test('memory navigation waits for the exact selected computer and bot and is consumed once', () => {
  clearPending()
  const sequence = islandThreadNavigation.open({ kind: 'memory', ref: 'computer/bot' })
  assert.equal(islandThreadNavigation.consume('computer/other'), null)
  assert.equal(islandThreadNavigation.consume('remote/bot'), null)
  const destination = islandThreadNavigation.consume('computer/bot')
  assert.deepEqual(destination, { kind: 'memory', ref: 'computer/bot', sequence })
  assert.equal(islandThreadNavigation.consume('computer/bot'), null)
})

test('existing terminal destination retains its exact session, backend and step until the target thread mounts', () => {
  clearPending()
  const backend: Backend = { id: 'claude', name: 'Claude', command: 'claude', available: true, installHint: '' }
  const sequence = islandThreadNavigation.open({ kind: 'terminal', ref: 'remote/bot', id: 'existing-session', backend, step: 'login' })
  assert.equal(islandThreadNavigation.consume('computer/bot'), null)
  const destination = islandThreadNavigation.consume('remote/bot')
  assert.equal(destination?.sequence, sequence)
  assert.equal(destination?.kind, 'terminal')
  if (destination?.kind === 'terminal') {
    assert.equal(destination.id, 'existing-session')
    assert.equal(destination.step, 'login')
    assert.equal(destination.backend?.id, 'claude')
  }
  assert.equal(islandThreadNavigation.pending, null)
})

test('a newer node-terminal destination replaces a pending memory request without requiring an agent backend', () => {
  clearPending()
  const old = islandThreadNavigation.open({ kind: 'memory', ref: 'computer/old' })
  const current = islandThreadNavigation.open({ kind: 'terminal', ref: 'computer/new', id: 'node-session', backend: null, step: 'node' })
  assert.ok(current > old)
  assert.equal(islandThreadNavigation.consume('computer/old'), null)
  const destination = islandThreadNavigation.consume('computer/new')
  assert.deepEqual(destination, { kind: 'terminal', ref: 'computer/new', id: 'node-session', backend: null, step: 'node', sequence: current })
  assert.equal(islandThreadNavigation.pending, null)
})
