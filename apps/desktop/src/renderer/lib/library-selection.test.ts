import assert from 'node:assert/strict'
import { test } from 'node:test'
import { marquee_after, selection_after, type SelectionModifiers } from './library-selection.ts'

const none: SelectionModifiers = { meta: false, shift: false }
const meta: SelectionModifiers = { meta: true, shift: false }
const shift: SelectionModifiers = { meta: false, shift: true }
const ids = ['a', 'b', 'c', 'd']

test('a plain click selects one item and leaves it as the range anchor', () => {
  assert.deepEqual(selection_after(['b', 'c'], 'a', 'd', none, ids), { ids: ['a'], anchor: 'a' })
})

test('Command-click toggles one item without dropping the others', () => {
  assert.deepEqual(selection_after(['a', 'b'], 'b', 'a', meta, ids), { ids: ['a'], anchor: 'b' })
  assert.deepEqual(selection_after(['a'], 'c', 'a', meta, ids), { ids: ['a', 'c'], anchor: 'c' })
})

test('Shift-click takes the range in visible order in either direction and keeps the first anchor', () => {
  assert.deepEqual(selection_after(['a'], 'd', 'b', shift, ids), { ids: ['b', 'c', 'd'], anchor: 'b' })
  assert.deepEqual(selection_after(['a'], 'b', 'd', shift, ids), { ids: ['b', 'c', 'd'], anchor: 'd' })
})

test('a range without a visible anchor starts at the clicked item', () => {
  assert.deepEqual(selection_after(['a'], 'c', 'gone', shift, ids), { ids: ['c'], anchor: 'c' })
})

test('a hidden click keeps only the still-visible selection, and clearing drops everything', () => {
  assert.deepEqual(selection_after(['a', 'b'], 'gone', 'a', none, ids), { ids: ['a', 'b'], anchor: null })
  assert.deepEqual(selection_after(['a', 'gone'], null, 'a', none, ids), { ids: ['a'], anchor: null })
})

test('a marquee replaces the selection, or adds to it when Command was held', () => {
  assert.deepEqual(marquee_after(['a'], ['b', 'c'], false), ['b', 'c'])
  assert.deepEqual(marquee_after(['a'], ['a', 'b', 'c'], true), ['a', 'b', 'c'])
})
