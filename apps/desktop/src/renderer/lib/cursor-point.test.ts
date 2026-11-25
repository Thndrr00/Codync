import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CURSOR_COORD_LIMIT, cursor_point } from './cursor-point.ts'

const display = { x: 1920, y: 0, width: 1280, height: 800 }

test('a move becomes a point on the display that owns it', () => {
  assert.deepEqual(cursor_point({ kind: 'move', x: 2000, y: 120 }, display), { x: 80, y: 120, click: false })
  assert.deepEqual(cursor_point({ kind: 'click', x: 2000.6, y: 120.4 }, display), { x: 81, y: 120, click: true })
})

test('an event outside the display or out of bound is not shown', () => {
  assert.equal(cursor_point({ kind: 'move', x: 100, y: 100 }, display), null)
  assert.equal(cursor_point({ kind: 'click', x: 3300, y: 100 }, display), null)
  assert.equal(cursor_point({ kind: 'move', x: CURSOR_COORD_LIMIT + 1, y: 0 }, display), null)
  assert.equal(cursor_point({ kind: 'move', x: 2000, y: -1 }, display), null)
  assert.equal(cursor_point({ kind: 'move', x: 2000, y: 900 }, display), null)
  assert.equal(cursor_point({ kind: 'move', x: Number.NaN, y: 0 }, display), null)
  assert.equal(cursor_point({ kind: 'move', x: 2000, y: 0 }, { ...display, width: 0 }), null)
})

test('a missing or unknown event is ignored', () => {
  assert.equal(cursor_point(null, display), null)
  assert.equal(cursor_point({ kind: 'move', x: 2000, y: 10 }, null), null)
  assert.equal(cursor_point({ kind: 'key', x: 2000, y: 10 } as never, display), null)
})
