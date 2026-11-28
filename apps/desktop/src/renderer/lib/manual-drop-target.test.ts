import assert from 'node:assert/strict'
import { test } from 'node:test'
import { manual_drop_target } from './manual-drop-target.ts'

test('a chat dropped at the top of its own folder stays in that folder', () => {
  assert.deepEqual(manual_drop_target('Work', 0), { folder: 'Work', position: 0 })
})

test('the insertion index is bounded and the folder is kept verbatim', () => {
  assert.deepEqual(manual_drop_target('  Work  ', 3.7), { folder: 'Work', position: 3 })
  assert.deepEqual(manual_drop_target('Work', -4), { folder: 'Work', position: 0 })
  assert.deepEqual(manual_drop_target('Work', Number.NaN), { folder: 'Work', position: 0 })
})

test('an empty folder is the root', () => {
  assert.deepEqual(manual_drop_target('   ', 2), { folder: '', position: 2 })
})
