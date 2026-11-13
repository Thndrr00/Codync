import assert from 'node:assert/strict'
import { test } from 'node:test'
import { lineDiff } from './doc-diff.ts'

test('identical text has only unchanged lines', () => {
  const rows = lineDiff('one\ntwo', 'one\ntwo')!
  assert.deepEqual(rows.map((row) => [row.kind, row.text]), [['same', 'one'], ['same', 'two']])
})

test('a changed line is shown removed and then added', () => {
  const rows = lineDiff('one\ntwo\nthree', 'one\nTWO\nthree')!
  assert.deepEqual(rows.map((row) => [row.kind, row.text]), [
    ['same', 'one'],
    ['removed', 'two'],
    ['added', 'TWO'],
    ['same', 'three'],
  ])
  assert.deepEqual([rows[1]!.oldNumber, rows[2]!.newNumber], [2, 2])
})

test('an inserted line keeps the old numbering', () => {
  const rows = lineDiff('one\nthree', 'one\ntwo\nthree')!
  assert.deepEqual(rows.map((row) => row.kind), ['same', 'added', 'same'])
  assert.deepEqual([rows[1]!.newNumber, rows[2]!.oldNumber, rows[2]!.newNumber], [2, 2, 3])
})

test('a document over the cap is not compared', () => {
  const huge = Array.from({ length: 1_300 }, (_, i) => `line ${i}`).join('\n')
  assert.equal(lineDiff(huge, 'short'), null)
})
