import assert from 'node:assert/strict'
import { test } from 'node:test'
import { providerRegistry, spentNote, spentWindow, usageStatus, windowTitle } from './usage-presentation.ts'

test('isolated Antigravity accounts keep the provider icon', () => {
  assert.equal(providerRegistry('antigravity:acc_one'), 'antigravity-acp')
  assert.equal(providerRegistry('antigravity:acc_two'), 'antigravity-acp')
  assert.equal(providerRegistry('future:account'), 'future')
})

test('model groups survive quota title formatting', () => {
  const window = { id: 'quota', percent: 4, resetsAt: null }
  assert.equal(windowTitle({ ...window, label: 'Gemini Models · Weekly Limit' }), 'Gemini Models · Weekly Limit')
  assert.equal(windowTitle({ ...window, label: 'Claude and GPT models · Weekly Limit' }), 'Claude and GPT models · Weekly Limit')
  assert.equal(windowTitle({ ...window, label: 'Weekly · Opus' }), 'Opus')
})

test('an unavailable account has an explanation instead of a quota number', () => {
  const provider = { id: 'antigravity', name: 'Antigravity', source: 'antigravity', updatedAt: 1, windows: [] }
  assert.equal(usageStatus({ ...provider, status: 'unsupported' }), 'Update this agent to read its plan limits.')
  assert.equal(usageStatus({ ...provider, status: 'unavailable' }), 'Plan limits are unavailable for this account.')
  assert.equal(usageStatus(provider), null)
})

/** A plan window: a share of a cap, unless `text` says it is a note instead. */
const plan = (percent: number, extra: { resetsAt?: number; text?: string } = {}) => ({
  id: 'weekly', label: 'Weekly · All models', percent, ...extra,
})

const account = (id: string, windows: ReturnType<typeof plan>[]) => ({
  id, name: 'Claude', source: 'claude', updatedAt: 1, windows,
})

test('a spent plan window stops that agent and says when it comes back', () => {
  const now = new Date('2026-01-01T00:00:00Z')
  const usage = { providers: [account('claude:acc_one', [plan(100, { resetsAt: now.getTime() + 200 * 60_000 })])] }
  assert.equal(spentWindow(usage, 'claude', now)?.percent, 100)
  assert.equal(spentNote(usage, 'claude', (text) => text, now), 'Usage limit reached · resets in 3h 20m')
  // Another agent's models are not stopped by this account.
  assert.equal(spentWindow(usage, 'codex', now), null)
  assert.equal(spentNote(usage, 'codex', (text) => text, now), null)
})

test('a window under its cap, a note, or one already reset does not stop models', () => {
  const now = new Date('2026-01-01T00:00:00Z')
  const under = { providers: [account('claude', [plan(99, { resetsAt: now.getTime() + 60_000 })])] }
  assert.equal(spentWindow(under, 'claude', now), null)
  const note = { providers: [account('claude', [plan(100, { text: '$4.20 left' })])] }
  assert.equal(spentWindow(note, 'claude', now), null)
  const reset = { providers: [account('claude', [plan(100, { resetsAt: now.getTime() - 60_000 })])] }
  assert.equal(spentWindow(reset, 'claude', now), null)
  // Without a known reset the window still counts: the chip just leaves the time out.
  const unknown = { providers: [account('claude', [plan(100)])] }
  assert.equal(spentNote(unknown, 'claude', (text) => text, now), 'Usage limit reached')
})

test('the most spent window is the one that speaks for the agent', () => {
  const now = new Date('2026-01-01T00:00:00Z')
  const usage = { providers: [account('claude', [plan(100, { resetsAt: now.getTime() + 60_000 }), { ...plan(100), id: 'opus', label: 'Weekly · Opus', percent: 120 }])] }
  assert.equal(spentWindow(usage, 'claude', now)?.label, 'Weekly · Opus')
})
