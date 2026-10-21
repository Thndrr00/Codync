import { test } from 'node:test'
import assert from 'node:assert/strict'
import { normalizeBot } from '../../shared/models.ts'
import { fastLabel, matchesRoster, rosterElapsed, rosterProject } from './roster-query.ts'

test('metadata search combines terms and filters without treating attention as approval', () => {
  const bot = normalizeBot({ id: 'a', name: 'Reviewer', cwd: 'C:\\work\\project', backend: 'codex', status: 'error' })
  assert.equal(matchesRoster(bot, 'reviewer codex project', 'attention'), true)
  assert.equal(matchesRoster(bot, 'another', 'attention'), false)
  assert.equal(matchesRoster(bot, '', 'working'), false)
  assert.equal(matchesRoster(bot, '', 'unread'), false)
})

test('project sections preserve declared paths and groups have no borrowed workspace', () => {
  assert.notEqual(rosterProject(normalizeBot({ id: 'a', cwd: '/a/project' })), rosterProject(normalizeBot({ id: 'b', cwd: '/b/project' })))
  assert.equal(rosterProject(normalizeBot({ id: 'g', kind: 'group', cwd: '/member/project' })), 'Group chats')
  assert.notEqual(rosterProject(normalizeBot({ id: 'a', managedWorkspace: true, cwd: '/personal/a' })), rosterProject(normalizeBot({ id: 'b', managedWorkspace: true, cwd: '/personal/b' })))
})

test('Fast requested and refused never claim active Fast', () => {
  assert.equal(fastLabel(normalizeBot({ id: 'a', fastMode: true })), 'Fast requested')
  assert.equal(fastLabel(normalizeBot({ id: 'a', fastMode: true, fastModeSupport: 'refused' })), 'Fast refused')
  assert.equal(fastLabel(normalizeBot({ id: 'g', kind: 'group', fastMode: true })), null)
})

test('duration uses host turn start and rejects missing or future timestamps', () => {
  assert.equal(rosterElapsed(null, 1_000), 'Duration unavailable')
  assert.equal(rosterElapsed(2_000, 1_000), 'Duration unavailable')
  assert.equal(rosterElapsed(1_000, 62_000), '1:01')
  assert.equal(rosterElapsed(1_000, 3_662_000), '1:01:01')
})
