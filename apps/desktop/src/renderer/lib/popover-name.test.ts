import assert from 'node:assert/strict'
import { test } from 'node:test'
import { popover_name } from './popover-name.ts'

test('the full name stays on one line and only a long title is truncated', () => {
  const name = 'Research assistant for the infrastructure team'
  const badge = popover_name(name, 'a very long purpose that runs past the badge width and keeps going')
  assert.equal(badge.name, name)
  assert.ok(badge.title.length <= 40)
  assert.ok(badge.title.endsWith('…'))
})

test('a short title is kept as written', () => {
  assert.deepEqual(popover_name('Brock', 'Store manager'), { name: 'Brock', title: 'Store manager' })
})

test('a missing title is an empty badge and a missing name is empty', () => {
  assert.deepEqual(popover_name('Brock', ''), { name: 'Brock', title: '' })
  assert.deepEqual(popover_name('  ', '  '), { name: '', title: '' })
})
