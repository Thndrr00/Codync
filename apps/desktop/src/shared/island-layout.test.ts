import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  defaultIslandControls, dockFor, islandPanelBounds, localIslandRect, normalizeIslandControls, snapIslandDock,
  type IslandDisplay, type IslandEdge,
} from './island-layout.ts'

const display: IslandDisplay = {
  id: 42, label: 'Left display',
  bounds: { x: -1920, y: -100, width: 1920, height: 1080 },
  workArea: { x: -1880, y: -60, width: 1880, height: 1000 },
}

test('invalid preferences receive independent defaults without prototype keys', () => {
  for (const value of [null, [], 'top', 42]) assert.deepEqual(normalizeIslandControls(value), defaultIslandControls)
  const input = JSON.parse('{"placement":"unknown","displayId":1e100,"locked":"false","density":"huge","docks":{"__proto__":{"edge":"left","offset":0},"constructor":{"edge":"left","offset":0},"42":{"edge":"left","offset":-2},"7":{"edge":"bottom","offset":3},"8":{"edge":"diagonal","offset":0}}}')
  assert.deepEqual(normalizeIslandControls(input), {
    ...defaultIslandControls,
    docks: { '42': { edge: 'left', offset: 0 }, '7': { edge: 'bottom', offset: 1 } },
  })
  const controls = normalizeIslandControls({ displayId: Infinity, docks: { 42: { edge: 'right', offset: NaN } } })
  assert.equal(controls.displayId, null)
  assert.deepEqual(controls.docks['42'], { edge: 'right', offset: 0.5 })
  assert.notEqual(normalizeIslandControls(null).docks, defaultIslandControls.docks)
})

test('saved controls preserve valid selections and use centered docks for replacement displays', () => {
  const controls = normalizeIslandControls({ placement: 'follow', displayId: -42, locked: false, density: 'expanded', docks: { '-42': { edge: 'right', offset: 0.25 } } })
  assert.equal(controls.placement, 'follow')
  assert.equal(controls.displayId, -42)
  assert.equal(controls.locked, false)
  assert.equal(controls.density, 'expanded')
  assert.deepEqual(dockFor(controls, -42), { edge: 'right', offset: 0.25 })
  assert.deepEqual(dockFor(controls, 99), { edge: 'top', offset: 0.5 })
  assert.deepEqual(dockFor(controls, -42), { edge: 'right', offset: 0.25 }, 'removing a monitor does not erase its saved dock')
})

test('all edge placements fit the usable area on monitors with negative coordinates', () => {
  const area = display.workArea
  for (const edge of ['top', 'bottom', 'left', 'right'] as const) {
    for (const offset of [0, 0.5, 1]) {
      const panel = islandPanelBounds(display, { edge, offset }, false)
      assert.ok(panel.x >= area.x && panel.y >= area.y)
      assert.ok(panel.x + panel.width <= area.x + area.width)
      assert.ok(panel.y + panel.height <= area.y + area.height)
      if (edge === 'top') assert.equal(panel.y, area.y)
      if (edge === 'bottom') assert.equal(panel.y + panel.height, area.y + area.height)
      if (edge === 'left') assert.equal(panel.x, area.x)
      if (edge === 'right') assert.equal(panel.x + panel.width, area.x + area.width)
    }
  }
  assert.equal(islandPanelBounds(display, { edge: 'top', offset: 0 }, false).x, area.x)
  const end = islandPanelBounds(display, { edge: 'top', offset: 1 }, false)
  assert.equal(end.x + end.width, area.x + area.width)
})

test('small work areas constrain both panel orientations instead of overflowing the screen', () => {
  const tiny = { ...display, workArea: { x: -100, y: 20, width: 24, height: 18 } }
  for (const edge of ['top', 'bottom', 'left', 'right'] as const) {
    assert.deepEqual(islandPanelBounds(tiny, { edge, offset: 0 }, false), { x: -100, y: 20, width: 24, height: 18 })
    assert.deepEqual(islandPanelBounds(tiny, { edge, offset: 1 }, false), { x: -100, y: 20, width: 24, height: 18 })
  }
})

test('only a centered top notch can overlap the menu area', () => {
  const notch = islandPanelBounds(display, { edge: 'top', offset: 0.5 }, true)
  assert.equal(notch.y, display.bounds.y)
  assert.equal(notch.x + notch.width / 2, display.bounds.x + display.bounds.width / 2)
  for (const edge of ['top', 'bottom', 'left', 'right'] as const) {
    for (const offset of [0, 1]) {
      assert.ok(islandPanelBounds(display, { edge, offset }, true).y >= display.workArea.y)
    }
  }
  assert.equal(islandPanelBounds(display, { edge: 'top', offset: 0.5 }, false).y, display.workArea.y)
})

test('snapping chooses each nearest edge and clamps its usable offset', () => {
  const area = display.workArea
  const center = { x: area.x + area.width / 2, y: area.y + area.height / 2 }
  const cases: [IslandEdge, { x: number; y: number }][] = [
    ['top', { ...center, y: area.y + 1 }],
    ['bottom', { ...center, y: area.y + area.height - 1 }],
    ['left', { ...center, x: area.x + 1 }],
    ['right', { ...center, x: area.x + area.width - 1 }],
  ]
  for (const [edge, point] of cases) assert.deepEqual(snapIslandDock(point, display, { width: 720, height: 380 }), { edge, offset: 0.5 })
  assert.deepEqual(snapIslandDock({ x: area.x + 1, y: area.y }, display, { width: 720, height: 380 }), { edge: 'top', offset: 0 })
  assert.deepEqual(snapIslandDock({ x: area.x + area.width - 1, y: area.y }, display, { width: 720, height: 380 }), { edge: 'top', offset: 1 })
  const tiny = { ...display, workArea: { x: 0, y: 0, width: 10, height: 10 } }
  assert.deepEqual(snapIslandDock({ x: 5, y: 0 }, tiny, { width: 10, height: 10 }), { edge: 'top', offset: 0.5 })
})

test('local hit rectangles follow all anchors and preserve zero-width side wake strips', () => {
  assert.deepEqual(localIslandRect({ width: 640, height: 196 }, 720, 380, 'top'), { x: 40, y: 0, width: 640, height: 196 })
  assert.deepEqual(localIslandRect({ width: 640, height: 196 }, 720, 380, 'bottom'), { x: 40, y: 184, width: 640, height: 196 })
  assert.deepEqual(localIslandRect({ width: 300, height: 400 }, 420, 560, 'left'), { x: 0, y: 80, width: 300, height: 400 })
  assert.deepEqual(localIslandRect({ width: 300, height: 400 }, 420, 560, 'right'), { x: 120, y: 80, width: 300, height: 400 })
  assert.deepEqual(localIslandRect({ width: 0, height: 160 }, 420, 560, 'left'), { x: 0, y: 200, width: 0, height: 160 })
  assert.deepEqual(localIslandRect({ width: 0, height: 160 }, 420, 560, 'right'), { x: 420, y: 200, width: 0, height: 160 })
  assert.deepEqual(localIslandRect({ width: 160, height: 0 }, 720, 380, 'bottom'), { x: 280, y: 380, width: 160, height: 0 })
  assert.deepEqual(localIslandRect({ width: 640, height: 196 }, 24, 18, 'top'), { x: 0, y: 0, width: 24, height: 18 })
})
