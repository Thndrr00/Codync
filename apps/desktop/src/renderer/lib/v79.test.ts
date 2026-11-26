import test from 'node:test'
import assert from 'node:assert/strict'
import { keepDraft, updateNotice, spokenRequest, stepErrorHint, portScale } from './v79.ts'

test('a new-agent draft keeps the chosen model and effort', () => {
  const saved = { model: 'gpt-5', effort: 'high', name: 'A' }
  const current = { model: '', effort: '', name: 'A' }
  assert.deepEqual(keepDraft(saved, current), { model: 'gpt-5', effort: 'high', name: 'A' })
  assert.equal(keepDraft(null, current), current)
  assert.equal(keepDraft(saved, null), saved)
})

test('only an idle update offers update settings', () => {
  assert.equal(updateNotice('idle'), 'Open update settings')
  assert.equal(updateNotice('required'), null)
})

test('internal instructions are not spoken words', () => {
  assert.equal(spokenRequest('# AGENTS.md instructions\nPlease summarize the file'), 'Please summarize the file')
  assert.equal(spokenRequest('hello'), 'hello')
})

test('a failed step names what to do', () => {
  assert.match(stepErrorHint('signin'), /Sign in again/)
  assert.match(stepErrorHint('field'), /optional/)
  assert.match(stepErrorHint('other'), /run it again/)
})

test('ports stay hit-friendly when zoomed out', () => {
  assert.equal(portScale(1), 1)
  assert.equal(portScale(0.5), 1)
  assert.equal(portScale(0.25), 2)
})
