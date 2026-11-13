import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseQuickCommand } from './quick-commands.ts'

const NOW = 1_767_225_600_000

test('ordinary messages and other slash commands are not quick commands', () => {
  assert.equal(parseQuickCommand('hello', NOW), null)
  assert.equal(parseQuickCommand('/fast', NOW), null)
  assert.equal(parseQuickCommand('/tasks are done', NOW), null)
})

test('a bare command is left to the slash menu', () => {
  assert.equal(parseQuickCommand('/task', NOW), null)
  assert.equal(parseQuickCommand('/task   ', NOW), null)
  assert.equal(parseQuickCommand('/schedule', NOW), null)
})

test('task takes the rest of the message as its title', () => {
  assert.deepEqual(parseQuickCommand('/task Review the PR', NOW), { kind: 'task', title: 'Review the PR' })
  assert.deepEqual(parseQuickCommand('/task   Ship it  ', NOW), { kind: 'task', title: 'Ship it' })
})

test('schedule reads minutes, hours or days and keeps the message', () => {
  assert.deepEqual(parseQuickCommand('/schedule 30m Buy milk', NOW), { kind: 'schedule', sendAt: NOW + 1_800_000, text: 'Buy milk' })
  assert.deepEqual(parseQuickCommand('/schedule 2h Send the report', NOW), { kind: 'schedule', sendAt: NOW + 7_200_000, text: 'Send the report' })
  assert.deepEqual(parseQuickCommand('/schedule 1d Follow up', NOW), { kind: 'schedule', sendAt: NOW + 86_400_000, text: 'Follow up' })
  assert.equal(parseQuickCommand('/schedule soon Buy milk', NOW)?.kind, 'error')
  assert.equal(parseQuickCommand('/schedule 30m', NOW)?.kind, 'error')
  assert.equal(parseQuickCommand('/schedule 0m Buy milk', NOW)?.kind, 'error')
})
