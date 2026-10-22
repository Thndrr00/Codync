import assert from 'node:assert/strict'
import { test } from 'node:test'
import { insertCodeBlock, quote, wrapInCode } from './composer-edits.ts'

test('wrapping a selection edits one range and keeps it selected', () => {
  assert.deepEqual(wrapInCode('run ls now', 4, 6), { start: 4, end: 6, replacement: '`ls`', selectionStart: 5, selectionEnd: 7 })
})

test('a reversed selection wraps the same as a forward one', () => {
  assert.deepEqual(wrapInCode('run ls now', 4, 6), wrapInCode('run ls now', 6, 4))
})

test('several lines become a fenced block on their own lines', () => {
  const edit = wrapInCode('a\nb', 0, 3)!
  assert.equal(edit.replacement, '```\na\nb\n```')
  assert.equal(edit.selectionStart, 4)
  assert.equal(edit.selectionEnd, 7)
})

test('an empty selection inserts a pair and a selection with a fence is refused', () => {
  assert.deepEqual(wrapInCode('hi', 2, 2), { start: 2, end: 2, replacement: '``', selectionStart: 3, selectionEnd: 3 })
  assert.equal(wrapInCode('```x```', 0, 7), null)
})

test('a block starts on a new line after text', () => {
  assert.equal(insertCodeBlock('hi', 2, 2)!.replacement, '\n```\n\n```')
})

test('quote prefixes each selected line', () => {
  assert.equal(quote('a\nb\nc', 0, 3).replacement, '> a\n> b')
})
