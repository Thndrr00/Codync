import assert from 'node:assert/strict'
import { test } from 'node:test'
import { importValidation } from './import-validation.ts'

test('a checked bot can be imported', () => {
  assert.equal(importValidation({ bots: ['a', 'b'], picked: ['b'], busy: false }), null)
})

test('nothing picked or an empty export names the exact reason', () => {
  assert.equal(importValidation({ bots: ['a', 'b'], picked: [], busy: false }), 'Pick at least one bot to import.')
  assert.equal(importValidation({ bots: [], picked: [], busy: false }), 'This file has no bots to import.')
})

test('a running import says to keep the window open', () => {
  assert.equal(importValidation({ bots: ['a'], picked: ['a'], busy: true }), 'Importing… keep this window open.')
})
