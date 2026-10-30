import { test } from 'node:test'
import assert from 'node:assert/strict'
import { RosterQuery } from '../store/roster-query.ts'
import type { RosterItem } from '../store/app-model'

function item(name: string, call: (method: string, body: Record<string, unknown>) => Promise<unknown>): RosterItem {
  return { bot: { id: name }, store: { hostName: name, ready: async () => ({ call }) } } as unknown as RosterItem
}
function answer(model: RosterQuery) {
  return new Promise<void>((resolve) => {
    const stop = model.subscribe(() => {
      if (model.result.kind === 'ready' || model.result.kind === 'error') { stop(); resolve() }
    })
  })
}

test('one unavailable computer does not discard another computer history', { timeout: 10_000 }, async () => {
  const model = new RosterQuery()
  const available = item('online', async () => ({ results: [{ id: 'message', botId: 'online', text: 'release', threadId: null, seq: 4 }], next: 4 }))
  const offline = item('offline', async () => { throw new Error('Disconnected') })
  const done = answer(model)
  model.find('release', [available, offline])
  await done
  assert.equal(model.result.kind, 'ready')
  if (model.result.kind === 'ready') {
    assert.equal(model.result.hits[0].hit.id, 'message')
    assert.match(model.result.warning ?? '', /offline/)
    assert.equal(model.result.cursors.get(available.store), 4)
  }
  model.dispose()
})

test('scope filtering probes older hosts instead of mislabelling unfiltered messages', { timeout: 10_000 }, async () => {
  const model = new RosterQuery()
  const methods: string[] = []
  const old = item('old host', async (method) => { methods.push(method); throw new Error('unknown method') })
  const done = answer(model)
  model.find('release', [old], false, 'threads')
  await done
  assert.deepEqual(methods, ['savedSearches'])
  assert.equal(model.result.kind, 'error')
  model.dispose()
})

test('pagination preserves previous rows and ignores repeated entries', { timeout: 10_000 }, async () => {
  const model = new RosterQuery()
  const hit = { id: 'message', botId: 'host', text: 'release', threadId: null, seq: 4 }
  const host = item('host', async () => ({ results: [hit], next: null }))
  let done = answer(model)
  model.find('release', [host]); await done
  done = answer(model)
  model.find('release', [host], true); await done
  assert.equal(model.result.kind, 'ready')
  if (model.result.kind === 'ready') assert.equal(model.result.hits.length, 1)
  model.dispose()
})

test('pagination retries computers missing from the first page until they recover', { timeout: 10_000 }, async () => {
  const model = new RosterQuery()
  let recovered = false
  let attempts = 0
  const intermittent = item('intermittent', async () => {
    attempts++
    if (!recovered) throw new Error('Disconnected')
    return { results: [{ id: 'late', botId: 'intermittent', text: 'release', threadId: null, seq: 5 }], next: null }
  })
  const online = item('online', async () => ({ results: [{ id: 'first', botId: 'online', text: 'release', threadId: null, seq: 4 }], next: 4 }))
  const sources = [intermittent, online]
  let done = answer(model)
  model.find('release', sources); await done
  done = answer(model)
  model.find('release', sources, true); await done
  assert.equal(attempts, 2)
  if (model.result.kind === 'ready') assert.match(model.result.warning ?? '', /intermittent/)
  else assert.fail('Successful computer results must remain visible')
  recovered = true
  done = answer(model)
  model.find('release', sources, true); await done
  assert.equal(attempts, 3)
  if (model.result.kind === 'ready') {
    assert.equal(model.result.warning, undefined)
    assert.deepEqual(model.result.hits.map(({ hit }) => hit.id), ['first', 'late'])
  } else assert.fail('Recovery must expose both computer results')
  model.dispose()
})
