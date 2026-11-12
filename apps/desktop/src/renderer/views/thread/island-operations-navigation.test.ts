import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { BotApp } from '../../../shared/bdots-models.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'

function clearPending() { const pending = islandThreadNavigation.pending; if (pending) islandThreadNavigation.consume(pending.ref) }

test('an operation destination cannot deliver capabilities to a replacement store with the same ref', () => {
  clearPending()
  const owner = {}, replacement = {}
  islandThreadNavigation.open({ kind: 'budget', ref: 'computer/bot', owner })
  assert.equal(islandThreadNavigation.consume('computer/bot', replacement), null)
  assert.equal(islandThreadNavigation.pending, null)
})

test('a matching operation owner receives its destination once after unrelated mounts', () => {
  clearPending()
  const owner = {}
  islandThreadNavigation.open({ kind: 'operationPanel', ref: 'computer/bot', owner, panel: 'trace' })
  assert.equal(islandThreadNavigation.consume('remote/bot', owner), null)
  assert.equal(islandThreadNavigation.consume('computer/bot', owner)?.kind, 'operationPanel')
  assert.equal(islandThreadNavigation.consume('computer/bot', owner), null)
})

test('budget navigation waits for the exact computer and bot and is consumed once', () => {
  clearPending()
  const sequence = islandThreadNavigation.open({ kind: 'budget', ref: 'computer/bot' })
  assert.equal(islandThreadNavigation.consume('computer/other'), null)
  assert.equal(islandThreadNavigation.consume('remote/bot'), null)
  assert.deepEqual(islandThreadNavigation.consume('computer/bot'), { kind: 'budget', ref: 'computer/bot', sequence })
  assert.equal(islandThreadNavigation.consume('computer/bot'), null)
})

test('mini-app navigation retains the private full object only for the matching conversation viewer', () => {
  clearPending()
  const app: BotApp = { slug: 'dashboard', botId: 'bot', title: 'Dashboard', entry: 'index.html', version: 1, createdAt: 1, updatedAt: 2, ownerToken: 'PRIVATE_OWNER', url: 'https://host/app?owner=PRIVATE_OWNER' }
  islandThreadNavigation.open({ kind: 'miniApp', ref: 'computer/bot', app })
  assert.equal(islandThreadNavigation.consume('remote/bot'), null)
  const destination = islandThreadNavigation.consume('computer/bot')
  assert.equal(destination?.kind, 'miniApp')
  if (destination?.kind === 'miniApp') {
    assert.equal(destination.app.slug, 'dashboard')
    assert.equal(destination.app.ownerToken, 'PRIVATE_OWNER')
    assert.equal(destination.app.url, app.url)
  }
})

test('document panel destinations preserve exact document identity and newer tools replace pending navigation', () => {
  clearPending()
  const previous = islandThreadNavigation.open({ kind: 'operationPanel', ref: 'computer/old', panel: 'trace' })
  const sequence = islandThreadNavigation.open({ kind: 'operationPanel', ref: 'computer/bot', panel: 'documents', documentId: 'document' })
  assert.ok(sequence > previous)
  assert.equal(islandThreadNavigation.consume('computer/old'), null)
  assert.deepEqual(islandThreadNavigation.consume('computer/bot'), { kind: 'operationPanel', ref: 'computer/bot', panel: 'documents', documentId: 'document', sequence })
  assert.equal(islandThreadNavigation.pending, null)
})
