import assert from 'node:assert/strict'
import { test } from 'node:test'
import { InspectorLog, redactEvent, redactValue } from './inspector.ts'

test('credentials never reach the panel, whatever their key', () => {
  const event = redactEvent({
    type: 'hello',
    token: 'host-bearer-123',
    apiKey: 'sk-live-123',
    authorization: 'Bearer abc',
    nested: { deviceSecret: 'pair-secret', name: 'Kitchen Mac' },
  })
  const text = JSON.stringify(event)
  assert.doesNotMatch(text, /host-bearer-123|sk-live-123|Bearer abc|pair-secret/)
  assert.match(text, /Kitchen Mac/)
  assert.equal(event.type, 'hello')
  assert.equal((event.data as { token: string }).token, '[redacted]')
})

test('prompt text, commands and working directories are shape only', () => {
  const event = redactEvent({
    type: 'entry',
    text: 'Please rotate the production certificates',
    command: 'rm -rf /tmp/build',
    cwd: '/Users/edwin/projects/secret-client',
    input: 'the user asked',
    output: 'the reply',
  })
  assert.deepEqual(event.data, {
    text: '[redacted]',
    command: '[redacted]',
    cwd: '[redacted]',
    input: '[redacted]',
    output: '[redacted]',
  })
})

test('a URL keeps its origin and path but loses its capability query', () => {
  const event = redactEvent({
    type: 'botApp',
    url: 'http://127.0.0.1:19222/apps/a/planner/secret-token/?x=1#frag',
    link: 'https://example.com/callback?code=abc',
  })
  assert.equal((event.data as { url: string }).url, 'http://127.0.0.1:19222/apps/a/planner/secret-token/')
  assert.equal((event.data as { link: string }).link, 'https://example.com/callback')
})

test('long strings are cut and lists are bounded', () => {
  const huge = 'a'.repeat(500)
  const value = redactValue({ note: huge, list: Array.from({ length: 30 }, (_, i) => i) }) as {
    note: string
    list: unknown[]
  }
  assert.equal(value.note.length, 121)
  assert.equal(value.list.length, 21)
  assert.equal(value.list[20], '+10')
})

test('the log keeps the newest events and clears on demand', () => {
  const log = new InspectorLog(3)
  for (let i = 0; i < 5; i++) log.add({ type: `t${i}` }, i)
  assert.deepEqual(log.all.map((event) => event.type), ['t2', 't3', 't4'])
  log.clear()
  assert.equal(log.all.length, 0)
})
