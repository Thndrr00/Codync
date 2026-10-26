import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { HostClient } from '../client/host-client'
import { SearchShortcuts } from '../store/search-shortcuts.ts'

const saved = { id: 'saved', query: 'release', scope: 'threads' as const }
function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => { resolve = done })
  return { promise, resolve }
}
function fixture(handle: (method: string, body: unknown) => Promise<unknown>) {
  const client = { call: handle } as Pick<HostClient, 'call'>
  return new SearchShortcuts(async () => client)
}

test('a failed save retains previously loaded searches and exposes recovery', async () => {
  const model = fixture(async (method) => {
    if (method === 'saveSearch') throw new Error('Host unavailable')
    return { searches: [saved] }
  })
  await model.load()
  await model.save('other', 'main')
  assert.deepEqual(model.searches, [saved])
  assert.equal(model.error, 'Host unavailable')
  assert.equal(model.busy, false)
  await model.load()
  assert.equal(model.error, null)
})

test('a remounted panel ignores the previous computer request', async () => {
  const old = deferred<unknown>()
  let reads = 0
  const model = fixture(async () => ++reads === 1 ? old.promise : { searches: [saved] })
  const first = model.load()
  await Promise.resolve()
  model.dispose(); model.resume()
  await model.load()
  old.resolve({ searches: [] }); await first
  assert.deepEqual(model.searches, [saved])
  assert.equal(model.loading, false)
})

test('duplicate clicks send one scoped save and a delete refreshes from the host', async () => {
  const pending = deferred<unknown>()
  const calls: { method: string; body: unknown }[] = []
  const model = fixture(async (method, body) => { calls.push({ method, body }); return method === 'saveSearch' ? pending.promise : { searches: [] } })
  const first = model.save('release', 'threads')
  await model.save('other', 'main')
  pending.resolve({ searches: [saved] }); await first
  assert.deepEqual(calls, [{ method: 'saveSearch', body: { query: 'release', scope: 'threads' } }])
  await model.remove(saved.id)
  assert.deepEqual(model.searches, [])
  assert.deepEqual(calls[1], { method: 'deleteSavedSearch', body: { id: 'saved' } })
})

test('an older host leaves shortcuts unavailable without pretending to save', async () => {
  const model = fixture(async () => { throw new Error('unknown method') })
  await model.load()
  assert.equal(model.available, false)
  assert.match(model.error ?? '', /unknown method/)
  assert.deepEqual(model.searches, [])
})
