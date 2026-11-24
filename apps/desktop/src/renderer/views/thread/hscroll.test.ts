import assert from 'node:assert/strict'
import { test } from 'node:test'
import { hscrollEdges, hscrollStep } from './hscroll.ts'

test('a block that fits needs no arrow', () => {
  assert.deepEqual(hscrollEdges(0, 400, 400), { left: false, right: false })
  assert.deepEqual(hscrollEdges(0, 400, 380), { left: false, right: false })
})

test('arrows follow the scroll position inside a wide block', () => {
  assert.deepEqual(hscrollEdges(0, 200, 900), { left: false, right: true })
  assert.deepEqual(hscrollEdges(300, 200, 900), { left: true, right: true })
  assert.deepEqual(hscrollEdges(700, 200, 900), { left: true, right: false })
})

test('the last pixel counts as the edge, not a missing arrow', () => {
  assert.deepEqual(hscrollEdges(698.5, 200, 900), { left: true, right: true })
  assert.deepEqual(hscrollEdges(700.5, 200, 900), { left: true, right: false })
})

test('an arrow press moves most of a viewport, never a sliver', () => {
  assert.equal(hscrollStep(300), 240)
  assert.equal(hscrollStep(50), 120)
})
