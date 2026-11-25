import assert from 'node:assert/strict'
import test from 'node:test'
import { limit_note } from './mcp-limit-note.ts'

test('a saved server note names the server and explains the working-directory limit', () => {
  const note = limit_note('filesystem')
  assert.match(note, /filesystem/)
  assert.match(note, /working directory/)
  assert.match(note, /relative path/)
})

test('a blank name still gets a fixed note', () => {
  const note = limit_note('  ')
  assert.match(note, /A saved server/)
  assert.match(note, /working directory/)
})
