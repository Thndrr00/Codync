import assert from 'node:assert/strict'
import { test } from 'node:test'
import { fromProfile, teamProfiles, type BotCatalog, type BotProfile } from './catalog.ts'

const profile = (id: string): BotProfile => ({
  id, name: id, summary: '', instructions: `${id} rules`, avatarColor: 'blue', avatarShape: 'blob', featured: false,
})

test('a profile fills the look and instructions but keeps the chosen agent and model', () => {
  const draft = { name: '', description: '', avatarColor: 'red', avatarShape: 'hex', backend: 'codex', model: 'gpt', cwd: '', permission: 'auto' }
  const next = fromProfile(draft, profile('writer'))
  assert.equal(next.name, 'writer')
  assert.equal(next.description, 'writer rules')
  assert.equal(next.avatarColor, 'blue')
  assert.equal(next.backend, 'codex')
  assert.equal(next.model, 'gpt')
})

test('a team lists its coordinator first and skips members the catalog lacks', () => {
  const catalog: BotCatalog = { bots: [profile('a'), profile('b')], teams: [] }
  const team = { id: 't', name: 'T', summary: '', purpose: '', coordinator: profile('lead'), members: ['b', 'gone', 'a'] }
  assert.deepEqual(teamProfiles(catalog, team).map((p) => p.id), ['lead', 'b', 'a'])
})
