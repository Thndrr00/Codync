import assert from 'node:assert/strict'
import { test } from 'node:test'
import { mcpFailureLines } from './mcp-failure-lines.ts'

test('a connection error wraps on words under the kind', () => {
  assert.deepEqual(mcpFailureLines({ kind: 'unreachable', message: 'Could not reach the server at all' }, 20), [
    'UNREACHABLE',
    'Could not reach the',
    'server at all',
  ])
})

test('a long token is broken so the row cannot grow', () => {
  const lines = mcpFailureLines({ kind: 'tool', message: 'Bad token abcdefghijklmnopqrstuvwxyz0123456789' }, 12)
  assert.deepEqual(lines.slice(0, 3), ['TOOL', 'Bad token', 'abcdefghijkl'])
  assert.ok(lines.every((line) => line.length <= 12))
})

test('nothing or an empty message shows no failure lines', () => {
  assert.deepEqual(mcpFailureLines(null), [])
  assert.deepEqual(mcpFailureLines(undefined), [])
  assert.deepEqual(mcpFailureLines({ kind: 'auth', message: '   ' }), ['AUTH'])
})
