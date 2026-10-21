import { test } from 'node:test'
import assert from 'node:assert/strict'
import { setTimeout as delay } from 'node:timers/promises'
import { RosterQuery, RosterStops, openRosterHit } from '../store/roster-query.ts'
import { normalizeBot } from '../../shared/models.ts'
import type { BotStore } from '../store/bot-store.ts'
import type { RosterItem } from '../store/app-model.ts'

function fixture(call: (method: string, body: unknown) => Promise<unknown>) {
  const bot = normalizeBot({ id: 'a', name: 'Reviewer', status: 'working', startedAt: 1000 })
  const store = { computer: { id: 'computer' }, contextId: 'account', ready: async () => ({ call }), bots: new Map([[bot.id, bot]]) } as unknown as BotStore
  const item = { bot, store, ref: { computerId: 'computer', botId: 'a' } } as RosterItem
  return { bot, store, item }
}

test('late search results cannot replace the current query or a disposed account', async () => {
  const pending: ((value: unknown) => void)[] = []
  const { item } = fixture(async () => new Promise((resolve) => pending.push(resolve)))
  const model = new RosterQuery()
  model.find('old', [item])
  await delay(340)
  model.find('new', [item])
  pending[0]!({ results: [{ id: 'old', botId: 'a', threadId: null, text: 'old', seq: 1 }], next: null })
  await delay(0)
  assert.equal(model.result.kind, 'loading')
  await delay(340)
  model.dispose()
  pending[1]!({ results: [], next: null })
  await delay(0)
  assert.equal(model.result.kind, 'loading')
})

test('request failures are errors rather than empty successful searches', async () => {
  const { item } = fixture(async () => { throw new Error('No permission') })
  const model = new RosterQuery()
  model.find('needle', [item])
  await delay(340)
  assert.equal(model.result.kind, 'error')
  model.dispose()
})

test('closing a preview discards an outstanding response and never marks read', async () => {
  const methods: string[] = []
  let finish: (value: unknown) => void = () => {}
  const { item } = fixture(async (method) => { methods.push(method); return new Promise((resolve) => { finish = resolve }) })
  const model = new RosterQuery()
  const load = model.showPreview(item)
  await delay(0)
  await model.showPreview(null)
  finish({ entries: [{ id: 'old' }] })
  await load
  assert.equal(model.preview, null)
  assert.deepEqual(methods, ['rosterPreview'])
})

test('a rejected stop leaves host status intact and clears pending state', async () => {
  const { store, bot } = fixture(async () => { throw new Error('Disconnected') })
  const stops = new RosterStops()
  await assert.rejects(stops.stop(store, bot), /Disconnected/)
  assert.equal(store.bots.get(bot.id)?.status, 'working')
  assert.equal(stops.pending.size, 0)
})

test('a retired search screen cannot apply a late message context', async () => {
  let applied = false
  const { store, item } = fixture(async () => ({ entries: [] }))
  store.openMessageContext = () => { applied = true }
  const opened = await openRosterHit(item, { id: 'e', botId: 'a', threadId: null, text: 'text', seq: 1 }, () => false)
  assert.equal(opened, false)
  assert.equal(applied, false)
})
