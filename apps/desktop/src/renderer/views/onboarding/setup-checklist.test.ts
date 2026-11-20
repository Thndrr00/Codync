import assert from 'node:assert/strict'
import { test } from 'node:test'
import { setupProgress, type SetupStepId } from './setup-checklist.ts'

const all = (except: Partial<Record<SetupStepId, boolean>> = {}): Record<SetupStepId, boolean> => ({
  host: true, access: true, bot: true, message: true, account: true, ...except,
})

test('progress counts only the steps still shown', () => {
  const progress = setupProgress(all({ bot: false, message: false }), ['account'])
  assert.deepEqual(progress.visible, ['host', 'access', 'bot', 'message'])
  assert.equal(progress.done, 2)
  assert.equal(progress.total, 4)
  assert.equal(progress.fraction, 0.5)
  assert.equal(progress.complete, false)
})

test('dismissing the last unfinished step completes the checklist', () => {
  const progress = setupProgress(all({ account: false }), ['account'])
  assert.equal(progress.complete, true)
  assert.equal(progress.fraction, 1)
})

test('dismissing every step leaves a complete, empty checklist', () => {
  const progress = setupProgress(all({ host: false }), ['host', 'access', 'bot', 'message', 'account'])
  assert.deepEqual(progress.visible, [])
  assert.equal(progress.total, 0)
  assert.equal(progress.complete, true)
})
