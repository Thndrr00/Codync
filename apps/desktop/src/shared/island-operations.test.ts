import assert from 'node:assert/strict'
import { test } from 'node:test'
import { islandAppFavoriteKey, islandContextPercent, type IslandRuntime } from './island-operations.ts'
import { validIslandAction } from './island-actions.ts'

const context = (used: number | null, size: number | null): IslandRuntime['context'] => ({ used, size, lastTurn: 1, compactions: 0 })

test('context use remains unknown when host counters or a valid window size are unavailable', () => {
  assert.equal(islandContextPercent(null), null)
  for (const [used, size] of [[null, 100], [0, null], [null, null], [-1, 100], [1, 0], [0, 0], [1, -100], [NaN, 100], [1, NaN], [Infinity, 100], [1, Infinity], [-Infinity, 100]]) {
    assert.equal(islandContextPercent(context(used, size)), null, `${used}/${size}`)
  }
})

test('context use preserves an actual zero and bounds finite host-reported consumption', () => {
  assert.equal(islandContextPercent(context(0, 100)), 0)
  assert.equal(islandContextPercent(context(1, 4)), 25)
  assert.equal(islandContextPercent(context(199, 200)), 100)
  assert.equal(islandContextPercent(context(200, 200)), 100)
  assert.equal(islandContextPercent(context(300, 200)), 100)
  assert.equal(islandContextPercent(context(Number.MAX_VALUE, Number.MIN_VALUE)), 100)
})

test('mini-app favorites retain the exact computer, bot and slug without delimiter collisions', () => {
  const key = islandAppFavoriteKey('computer/bot', 'dashboard')
  assert.deepEqual(JSON.parse(key), ['computer/bot', 'dashboard'])
  assert.equal(islandAppFavoriteKey('computer/bot', 'dashboard'), key)
  for (const [ref, slug] of [['remote/bot', 'dashboard'], ['computer/other', 'dashboard'], ['computer/bot', 'other'], ['computer/bot', '__proto__']]) {
    assert.notEqual(islandAppFavoriteKey(ref, slug), key)
  }
  assert.notEqual(islandAppFavoriteKey('computer/bot/one', 'two'), islandAppFavoriteKey('computer/bot', 'one/two'))
  assert.notEqual(islandAppFavoriteKey('computer/bot', '"a,b"'), islandAppFavoriteKey('computer/bot', 'a,b'))
})

test('operation intents expose only typed navigation and discard arbitrary commands, paths and trust decisions', () => {
  const ref = 'computer/bot'
  const extras = { command: 'arbitrary shell command', path: '/private/file', url: 'https://untrusted.test', trustHostKey: true, capability: 'PRIVATE_CAPABILITY' }
  for (const kind of ['loadOperations', 'openExistingBrowser'] as const) assert.deepEqual(validIslandAction({ kind, ref, ...extras }), { kind, ref })
  for (const kind of ['openOperationDocument', 'openOperationApp', 'selectWorktreeBot'] as const) assert.deepEqual(validIslandAction({ kind, ref, id: 'known-id', ...extras }), { kind, ref, id: 'known-id' })
  assert.deepEqual(validIslandAction({ kind: 'retryIslandSSH', id: 'known-profile', ...extras }), { kind: 'retryIslandSSH', id: 'known-profile' })
  assert.deepEqual(validIslandAction({ kind: 'reviewDevices', accept: true, ...extras }), { kind: 'reviewDevices' })
  for (const kind of ['runShell', 'gitCheckout', 'createBrowser', 'acceptPairing', 'arbitraryHostCall']) assert.equal(validIslandAction({ kind, ref, id: 'known-id', ...extras }), null)
})

test('operation panels and settings accept only their named existing destinations', () => {
  for (const panel of ['trace', 'computer', 'budget'] as const) assert.deepEqual(validIslandAction({ kind: 'openOperationPanel', ref: 'computer/bot', panel, method: 'arbitrary' }), { kind: 'openOperationPanel', ref: 'computer/bot', panel })
  for (const page of ['updates', 'computers', 'channels'] as const) assert.deepEqual(validIslandAction({ kind: 'openOperationSettings', page, method: 'install' }), { kind: 'openOperationSettings', page })
  for (const destination of [null, undefined, '', 'shell', '/private/file', 'file:///private/file', 'https://untrusted.test', {}, ['updates']]) {
    assert.equal(validIslandAction({ kind: 'openOperationSettings', page: destination }), null)
    assert.equal(validIslandAction({ kind: 'openOperationPanel', ref: 'computer/bot', panel: destination }), null)
  }
})

test('operation resource identities require bounded IDs and complete bot references', () => {
  for (const kind of ['openOperationDocument', 'openOperationApp', 'selectWorktreeBot'] as const) {
    for (const id of [undefined, null, '', 1, {}, 'x'.repeat(257)]) assert.equal(validIslandAction({ kind, ref: 'computer/bot', id }), null)
    assert.notEqual(validIslandAction({ kind, ref: 'computer/bot', id: 'x'.repeat(256) }), null)
    for (const ref of [undefined, null, '', 'computer', '/bot', 'computer/', `pc/${'x'.repeat(510)}`]) assert.equal(validIslandAction({ kind, ref, id: 'known' }), null)
  }
  assert.notEqual(validIslandAction({ kind: 'loadOperations', ref: `pc/${'x'.repeat(509)}` }), null)
  assert.equal(validIslandAction({ kind: 'retryIslandSSH', id: 'x'.repeat(257) }), null)
  assert.equal(validIslandAction({ kind: 'openExistingBrowser', ref: 'missing' }), null)
})
