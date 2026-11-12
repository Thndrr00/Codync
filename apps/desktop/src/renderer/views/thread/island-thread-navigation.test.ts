import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, SharedFile } from '../../../shared/models.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'

function clear() {
  const current = islandThreadNavigation.pending
  if (current) islandThreadNavigation.consume(current.ref)
}

test('artifact navigation survives unrelated mounts until the exact computer and bot mount', () => {
  clear()
  const sequence = islandThreadNavigation.open({ kind: 'artifact', ref: 'computer/bot', artifactId: 'artifact' })
  assert.equal(islandThreadNavigation.consume('other-computer/bot'), null)
  assert.equal(islandThreadNavigation.consume('computer/other-bot'), null)
  assert.equal(islandThreadNavigation.pending?.sequence, sequence)
  assert.deepEqual(islandThreadNavigation.consume('computer/bot'), { kind: 'artifact', ref: 'computer/bot', artifactId: 'artifact', sequence })
  assert.equal(islandThreadNavigation.pending, null)
  assert.equal(islandThreadNavigation.consume('computer/bot'), null)
})

test('a newer destination replaces an older request without allowing its mount to consume the newer one', () => {
  clear()
  const previous = islandThreadNavigation.open({ kind: 'artifact', ref: 'computer/a', artifactId: 'first' })
  const sequence = islandThreadNavigation.open({ kind: 'artifact', ref: 'computer/b', artifactId: 'second' })
  assert.ok(sequence > previous)
  assert.equal(islandThreadNavigation.consume('computer/a'), null)
  const destination = islandThreadNavigation.consume('computer/b')
  assert.equal(destination?.kind, 'artifact')
  if (destination?.kind === 'artifact') assert.equal(destination.artifactId, 'second')
})

test('published file navigation carries its exact source entry into the matching conversation once', () => {
  clear()
  const file: SharedFile = { id: 'file', name: 'report.txt', size: 3, sha256: 'sha' }
  const entry: Entry = { id: 'entry', botId: 'bot', seq: 1, rev: 1, kind: 'agent', turn: 1, data: { files: [file] }, createdAt: 1, updatedAt: 1 }
  const sequence = islandThreadNavigation.open({ kind: 'file', ref: 'computer/bot', file, entry, entryId: entry.id })
  assert.equal(islandThreadNavigation.consume('computer/other'), null)
  assert.deepEqual(islandThreadNavigation.consume('computer/bot'), { kind: 'file', ref: 'computer/bot', file, entry, entryId: entry.id, sequence })
  assert.equal(islandThreadNavigation.consume('computer/bot'), null)
})
