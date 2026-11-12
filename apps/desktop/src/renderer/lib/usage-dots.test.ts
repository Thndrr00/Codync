import assert from 'node:assert/strict'
import { test } from 'node:test'
import { usageDotGrid, usagePercent } from './usage-dots.ts'

test('zero usage is empty and invalid percentages cannot fill or overflow the meter', () => {
  for (const value of [0, -3, NaN, Infinity, -Infinity]) {
    assert.equal(usagePercent(value), 0)
    assert.equal(usageDotGrid(60, 8, value).filled, 0)
  }
  assert.equal(usagePercent(125), 100)
  assert.equal(usageDotGrid(60, 8, 125).filled, 15)
  assert.equal(usagePercent(0.5), 0.5)
})

test('tray and widget meters fill whole columns without changing the shape of their dots', () => {
  const empty = usageDotGrid(60, 8, 0)
  const half = usageDotGrid(60, 8, 50)
  const full = usageDotGrid(60, 8, 100)
  assert.deepEqual(empty, { columns: 15, rows: 2, stepX: 4, stepY: 4, radius: 1.25, filled: 0 })
  assert.equal(half.filled, 8)
  assert.equal(full.filled, 15)
  assert.equal(half.radius, full.radius)
})

test('dots stay inside narrow and fractional containers', () => {
  for (const [width, height] of [[0, 0], [1, 2], [28, 8], [103.5, 13]]) {
    const grid = usageDotGrid(width, height, 62)
    assert.ok(grid.radius <= Math.min(grid.stepX, grid.stepY) / 2)
    assert.ok((grid.columns - 0.5) * grid.stepX + grid.radius <= width)
    assert.ok((grid.rows - 0.5) * grid.stepY + grid.radius <= height)
    assert.ok(grid.filled >= 0 && grid.filled <= grid.columns)
  }
})
