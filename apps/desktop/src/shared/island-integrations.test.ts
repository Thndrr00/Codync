import assert from 'node:assert/strict'
import { test } from 'node:test'
import { islandApprovalAge, safeIslandGitHubUrl } from './island-integrations.ts'
import { validIslandAction } from './island-actions.ts'

test('GitHub links require the exact HTTPS host without credentials or nondefault ports', () => {
  assert.equal(safeIslandGitHubUrl('https://github.com/owner/repo/pull/42'), true)
  assert.equal(safeIslandGitHubUrl('https://github.com/owner/repo/actions/runs/42?check_suite_focus=true#jobs'), true)
  for (const value of [null, 1, {}, '', 'javascript:alert(1)', 'http://github.com/a/b', 'https://api.github.com/a/b', 'https://github.com.attacker.test/a/b', 'https://attacker.test@github.com/a/b', 'https://github.com:8443/a/b', 'https://github.com./a/b', 'https://github.com@attacker.test/a/b', `https://github.com/${'a'.repeat(2048)}`]) assert.equal(safeIslandGitHubUrl(value), false, String(value))
})

test('approval age uses host creation time and keeps missing, invalid or future timestamps unknown', () => {
  assert.equal(islandApprovalAge(60_000, 185_000), 2)
  assert.equal(islandApprovalAge(185_000, 185_000), 0)
  for (const createdAt of [undefined, NaN, Infinity, -1, 0, 185_001]) assert.equal(islandApprovalAge(createdAt, 185_000), null)
  assert.equal(islandApprovalAge(60_000, NaN), null)
  assert.equal(islandApprovalAge(60_000, Infinity), null)
})

test('integration actions allow only typed intents and discard unrelated payload fields', () => {
  const ref = 'computer/bot'
  for (const kind of ['loadIntegrations', 'reconnectComputer', 'openMemory'] as const) assert.deepEqual(validIslandAction({ kind, ref, path: '/private' }), { kind, ref })
  for (const kind of ['openAgentTerminal', 'openConnector', 'openIntegrationArtifact', 'runSavedSearch', 'openBookmark', 'openSearchHit'] as const) assert.deepEqual(validIslandAction({ kind, ref, id: 'known-id', command: 'arbitrary' }), { kind, ref, id: 'known-id' })
  assert.deepEqual(validIslandAction({ kind: 'stopDictation', id: 'dictation-1', ref, text: 'private' }), { kind: 'stopDictation', id: 'dictation-1' })
  assert.deepEqual(validIslandAction({ kind: 'openGitHubLink', ref, url: 'https://github.com/o/r/pull/1', token: 'PRIVATE' }), { kind: 'openGitHubLink', ref, url: 'https://github.com/o/r/pull/1' })
  for (const action of [{ kind: 'arbitraryHostCall', ref }, { kind: 'openConnector', ref }, { kind: 'openMemory', ref: 'missing-separator' }, { kind: 'openBookmark', ref, id: '' }, { kind: 'openAgentTerminal', ref, id: 'x'.repeat(257) }, { kind: 'stopDictation', id: '' }, { kind: 'openGitHubLink', ref, url: 'https://evil.test/o/r' }]) assert.equal(validIslandAction(action), null)
})

test('quick capture validates Unicode character limits and a bounded correlation id', () => {
  const base = { kind: 'appendScratchpad', ref: 'computer/bot', requestId: 'capture-1' }
  const text = '🙂'.repeat(4000)
  assert.deepEqual(validIslandAction({ ...base, text, token: 'PRIVATE' }), { ...base, text })
  for (const action of [{ ...base, text: '🙂'.repeat(4001) }, { ...base, text: ' \n\t ' }, { ...base, text: 'note', requestId: '' }, { ...base, text: 'note', requestId: 'x'.repeat(129) }, { ...base, text: null }, { ...base, text: 'note', ref: 'invalid' }]) assert.equal(validIslandAction(action), null)
  assert.notEqual(validIslandAction({ ...base, text: 'note', requestId: 'x'.repeat(128) }), null)
})
