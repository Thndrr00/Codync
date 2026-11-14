import { strict as assert } from 'node:assert'
import { test } from 'node:test'
import { MirrorCache, type MirrorCacheBackend } from './mirror-cache.ts'

function memoryBackend(seed: Record<string, string> = {}) {
  const values = new Map(Object.entries(seed))
  const backend: MirrorCacheBackend & { values: Map<string, string> } = {
    values,
    read: async (key) => values.get(key) ?? null,
    write: async (key, text) => void values.set(key, text),
    remove: async (key) => void values.delete(key),
  }
  return backend
}

function memoryStorage(seed: Record<string, string> = {}) {
  const values = new Map(Object.entries(seed))
  const storage = {
    get length() {
      return values.size
    },
    key: (index: number) => [...values.keys()][index] ?? null,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
    removeItem: (key: string) => void values.delete(key),
    clear: () => values.clear(),
  }
  return storage as Storage
}

test('a plaintext cache from an older build moves into the encrypted one once', async () => {
  const backend = memoryBackend()
  const storage = memoryStorage({ 'codync-mirror-local-a': '{"rev":3}' })
  const cache = new MirrorCache(backend, storage)
  assert.equal(await cache.read('codync-mirror-local-a'), '{"rev":3}')
  assert.equal(backend.values.get('codync-mirror-local-a'), '{"rev":3}')
  assert.equal(storage.getItem('codync-mirror-local-a'), null, 'the plain copy is gone')
  assert.equal(await cache.read('codync-mirror-local-a'), '{"rev":3}')
})

test('writes go to the encrypted cache and never leave plaintext behind', async () => {
  const backend = memoryBackend()
  const storage = memoryStorage({ 'codync-mirror-local-a': 'old' })
  const cache = new MirrorCache(backend, storage)
  await cache.write('codync-mirror-local-a', 'new')
  assert.equal(backend.values.get('codync-mirror-local-a'), 'new')
  assert.equal(storage.getItem('codync-mirror-local-a'), null)
  await cache.remove('codync-mirror-local-a')
  assert.equal(backend.values.has('codync-mirror-local-a'), false)
})

test('a broken cache falls back to what the window still holds', async () => {
  const backend = memoryBackend()
  backend.read = async () => {
    throw new Error('unreadable')
  }
  const cache = new MirrorCache(backend, memoryStorage({ k: 'kept' }))
  assert.equal(await cache.read('k'), 'kept')
})
