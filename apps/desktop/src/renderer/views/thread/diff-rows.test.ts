import assert from 'node:assert/strict'
import { test } from 'node:test'
import { looksLikeDiff, parseDiff } from './diff-rows.ts'

test('diff rows number lines only inside the hunk ranges', () => {
  const rows = parseDiff('diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -3,2 +3,2 @@\n ctx\n-old\n+new\n+extra')!
  assert.equal(rows[1]!.kind, 'header')
  assert.equal(rows[1]!.raw, '--- a/f')
  assert.deepEqual(rows.slice(4), [
    { kind: 'context', raw: ' ctx', oldNumber: 3, newNumber: 3 },
    { kind: 'removed', raw: '-old', oldNumber: 4 },
    { kind: 'added', raw: '+new', newNumber: 4 },
    { kind: 'added', raw: '+extra', newNumber: null },
  ])
})

test('text that is not a diff stays text', () => {
  assert.equal(parseDiff('just some words'), null)
  assert.equal(parseDiff('+added but no hunk header\nplain'), null)
  assert.equal(looksLikeDiff('diff', ''), true)
  assert.equal(looksLikeDiff('kotlin', 'diff --git a b'), false)
})

test('binary diffs keep their header without invented lines', () => {
  assert.deepEqual(parseDiff('diff --git a/x.png b/x.png\nBinary files a/x.png and b/x.png differ'), [
    { kind: 'header', raw: 'diff --git a/x.png b/x.png' },
    { kind: 'header', raw: 'Binary files a/x.png and b/x.png differ' },
  ])
})

test('a huge diff falls back to plain text', () => {
  const big = Array.from({ length: 2001 }, (_, i) => ` line ${i}`).join('\n')
  assert.equal(parseDiff(big), null)
})
