import { strict as assert } from 'node:assert'
import { test } from 'node:test'
import { cacheFileName } from './cache-key.ts'

test('cache files are named by a stable hash, never by the account or computer', () => {
  const key = 'codync-mirror-alice-0b6f-4b7f-9d1a'
  const name = cacheFileName(key)
  assert.match(name, /^[0-9a-f]{16}\.bin$/)
  assert.equal(name, cacheFileName(key), 'stable across calls and launches')
  assert.equal(name.includes('alice'), false)
  assert.notEqual(name, cacheFileName('codync-mirror-bob-0b6f-4b7f-9d1a'))
  assert.notEqual(cacheFileName('local/computer-a'), cacheFileName('local/computer-b'))
})
