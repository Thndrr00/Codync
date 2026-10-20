import assert from 'node:assert/strict'
import { test } from 'node:test'
import { encodeTemplate } from './bot-template.ts'
import { draftOf, normalizeBot } from './models.ts'

test('portable templates omit the host account while editing preserves it', () => {
  const draft = draftOf(normalizeBot({
    id: 'b', name: 'Reviewer', backend: 'antigravity', account: 'acc_local',
    model: 'chosen', pinned: true, hidden: true,
  }))
  const exported = JSON.parse(encodeTemplate(draft))
  for (const key of ['id', 'pinned', 'hidden', 'account']) assert.equal(key in exported, false)
  assert.equal(exported.model, 'chosen')
  assert.equal(draft.account, 'acc_local')
})
