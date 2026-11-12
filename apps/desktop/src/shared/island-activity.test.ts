import assert from 'node:assert/strict'
import { test } from 'node:test'
import { defaultUsagePreferences, pinnedFirst, usageNotice, type IslandUsagePreferences } from './island-activity.ts'
import { validIslandAction } from './island-actions.ts'

const now = 1_000_000
const preferences = (change: Partial<IslandUsagePreferences> = {}): IslandUsagePreferences => ({
  ...defaultUsagePreferences, notices: true, ...change,
})
const report = (percent: number, resetsAt: number | null = null, text?: string | null) => [{
  id: 'provider', name: 'Provider', windows: [{ id: 'session', percent, resetsAt, text }],
}]

test('pins retain all items and preserve the original unpinned order without mutating inputs', () => {
  const items = Object.freeze([{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }])
  const pins = Object.freeze(['c', 'a', 'missing'])
  assert.deepEqual(pinnedFirst(items, pins, (item) => item.id).map((item) => item.id), ['c', 'a', 'b', 'd'])
  assert.deepEqual(items.map((item) => item.id), ['a', 'b', 'c', 'd'])
  assert.deepEqual(pins, ['c', 'a', 'missing'])
  assert.deepEqual(pinnedFirst(items, [], (item) => item.id), items)
  assert.deepEqual(pinnedFirst([], pins, (item: { id: string }) => item.id), [])
})

test('usage notices respect opt-out and exclude readings that are notes or not finite limits', () => {
  assert.equal(usageNotice(report(100), preferences({ notices: false }), now), null)
  assert.equal(usageNotice(report(99, now + 60_000, 'Unlimited'), preferences(), now), null)
  for (const percent of [NaN, Infinity, -Infinity]) {
    assert.equal(usageNotice(report(percent, now + 60_000), preferences(), now), null)
  }
  assert.equal(usageNotice([], preferences(), now), null)
})

test('the highest exhausted allowance wins over lower limits and upcoming resets', () => {
  const providers = [
    { id: 'a', name: 'Alpha', windows: [{ id: 'week', percent: 90, resetsAt: null }] },
    { id: 'b', name: 'Beta', windows: [{ id: 'session', percent: 97, resetsAt: now + 90_000 }] },
    { id: 'c', name: 'Gamma', windows: [{ id: 'session', percent: 20, resetsAt: now + 60_000 }] },
  ]
  assert.deepEqual(usageNotice(providers, preferences(), now), {
    kind: 'limit', provider: 'Beta', percent: 97, resetsAt: now + 90_000,
  })
  assert.equal(usageNotice(report(89), preferences(), now), null)
  assert.deepEqual(usageNotice(report(70), preferences({ threshold: 70 }), now), {
    kind: 'limit', provider: 'Provider', percent: 70, resetsAt: null,
  })
})

test('reset notices require some usage and a future reset within fifteen minutes', () => {
  assert.deepEqual(usageNotice(report(20, now + 15 * 60_000), preferences(), now), {
    kind: 'reset', provider: 'Provider', percent: 20, resetsAt: now + 15 * 60_000,
  })
  for (const resetsAt of [null, now - 1, now, now + 15 * 60_000 + 1]) {
    assert.equal(usageNotice(report(20, resetsAt), preferences(), now), null)
  }
  assert.equal(usageNotice(report(0, now + 60_000), preferences(), now), null)
  const providers = [
    { id: 'later', name: 'Later', windows: [{ id: 'week', percent: 30, resetsAt: now + 120_000 }] },
    { id: 'first', name: 'First', windows: [{ id: 'session', percent: 20, resetsAt: now + 60_000 }] },
  ]
  assert.equal(usageNotice(providers, preferences(), now)?.provider, 'First')
})

test('island intents use a whitelist and drop all unexpected fields', () => {
  for (const kind of ['refreshUsage', 'refreshActivity', 'openChannels', 'openDiagnostics']) {
    assert.deepEqual(validIslandAction({ kind, ref: 'ignored/bot', method: 'deleteAll', auth: 'secret' }), { kind })
  }
  for (const kind of ['callMute', 'callEnd']) {
    assert.deepEqual(validIslandAction({ kind, callId: 'call-1', auth: 'secret' }), { kind, callId: 'call-1' })
    assert.equal(validIslandAction({ kind }), null)
    assert.equal(validIslandAction({ kind, callId: '' }), null)
  }
  for (const kind of ['stopBot', 'openFlow']) {
    assert.deepEqual(validIslandAction({ kind, ref: 'computer/bot', method: 'deleteAll', auth: 'secret' }), { kind, ref: 'computer/bot' })
  }
  for (const input of [null, undefined, 4, 'stopBot', {}, { kind: 'deleteBot' }, { kind: 'call' }]) {
    assert.equal(validIslandAction(input), null)
  }
})

test('bot intents reject missing, malformed or oversized conversation references', () => {
  for (const kind of ['stopBot', 'openFlow']) {
    for (const ref of [undefined, null, 5, '', 'computer', '/bot', 'computer/', `computer/${'b'.repeat(513)}`]) {
      assert.equal(validIslandAction({ kind, ref }), null)
    }
  }
})
