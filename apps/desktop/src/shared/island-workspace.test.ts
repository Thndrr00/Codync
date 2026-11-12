import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { IslandBot } from './island'
import { elapsedIslandRun, inIslandScope, normalizeIslandHistory, searchIslandBots, updateIslandHistory } from './island-workspace.ts'
import { normalizeIslandControls } from './island-layout.ts'
import { validIslandAction } from './island-actions.ts'

const bots = [
  { ref: 'pc/a', name: 'José', folder: 'app', project: '/one/app', backend: 'Claude', status: 'idle' },
  { ref: 'pc/b', name: 'Review', folder: 'app', project: '/one/app', backend: 'Codex', status: 'working' },
  { ref: 'pc/c', name: 'Other', folder: 'app', project: '/two/app', backend: 'Codex', status: 'idle' },
  { ref: 'remote/d', name: 'Remote', folder: 'app', project: '/one/app', backend: 'Codex', status: 'idle' },
] as IslandBot[]

test('island search matches all terms, accents and full project paths', () => {
  assert.deepEqual(searchIslandBots(bots, ' JOSE claude ').map((bot) => bot.ref), ['pc/a'])
  assert.deepEqual(searchIslandBots(bots, '/two/app codex').map((bot) => bot.ref), ['pc/c'])
  assert.equal(searchIslandBots(bots, 'missing').length, 0)
  assert.equal(searchIslandBots(bots, '').length, 4)
})

test('activity project scope never conflates folder basenames or computers', () => {
  assert.deepEqual(bots.filter((bot) => inIslandScope(bot.ref, 'project', bots[0]!, bots)).map((bot) => bot.ref), ['pc/a', 'pc/b'])
  assert.equal(inIslandScope('pc/b', 'bot', bots[0]!, bots), false)
  assert.equal(inIslandScope('pc/a', 'project', null, bots), false)
  assert.equal(inIslandScope('pc/a', 'all', null, bots), true)
})

test('notice groups count occurrences, preserve active dismissals and bound storage', () => {
  const notice = { key: 'error:disk', text: 'Disk full' }
  const first = updateIslandHistory({ items: [], active: [] }, [notice, notice], 1)
  assert.equal(first.items[0]!.count, 1)
  assert.deepEqual(updateIslandHistory(first, [notice], 2), first)
  const cleared = { ...first, items: [] }
  assert.equal(updateIslandHistory(cleared, [notice], 3).items.length, 0)
  const ended = updateIslandHistory(first, [], 4)
  const repeated = updateIslandHistory(ended, [notice], 5)
  assert.equal(repeated.items[0]!.count, 2)
  assert.equal(repeated.items[0]!.firstAt, 1)
  assert.equal(repeated.items[0]!.at, 5)
  assert.equal(updateIslandHistory(ended, Array.from({ length: 220 }, (_, key) => ({ key: String(key), text: 'notice' })), 6).items.length, 50)
  assert.deepEqual(normalizeIslandHistory({ items: [null, { key: 1 }], active: [1] }), { items: [], active: [] })
})

test('elapsed time requires a real host start and valid finish', () => {
  assert.equal(elapsedIslandRun(null, null, 10_000), null)
  assert.equal(elapsedIslandRun(1000, null, 4500), 3)
  assert.equal(elapsedIslandRun(1000, 3000, 99_000), 2)
  assert.equal(elapsedIslandRun(5000, 3000, 99_000), null)
  assert.equal(elapsedIslandRun(NaN, null, 5000), null)
})

test('island display preferences reject corrupt data and keep independent text bounds', () => {
  const preferences = normalizeIslandControls({ quiet: 'true', quietFullscreen: true, fontSize: 99, highContrast: true })
  assert.equal(preferences.quiet, false)
  assert.equal(preferences.quietFullscreen, true)
  assert.equal(preferences.fontSize, 18)
  assert.equal(preferences.highContrast, true)
  assert.equal(normalizeIslandControls({ fontSize: NaN }).fontSize, 13)
})

test('workspace intents reject arbitrary resources and strip extra payload fields', () => {
  assert.deepEqual(validIslandAction({ kind: 'openArtifact', ref: 'pc/bot', id: 'id', url: 'file:///secret' }), { kind: 'openArtifact', ref: 'pc/bot', id: 'id' })
  for (const action of [{ kind: 'retryRun', ref: 'pc/bot', id: '' }, { kind: 'cancelQueued', ref: 'missing', id: 'x' }, { kind: 'openFile', ref: 'pc/bot', id: 'x' }]) assert.equal(validIslandAction(action), null)
})
