/** Placement is device UI state; the host still owns every action shown by the island. */
export type IslandEdge = 'top' | 'bottom' | 'left' | 'right'
export type IslandDensity = 'compact' | 'expanded'

export interface IslandDock {
  edge: IslandEdge
  /** Position along the edge's available span, after allowing room for the panel. */
  offset: number
}

export interface IslandControls {
  placement: 'automatic' | 'follow' | 'primary'
  displayId: number | null
  locked: boolean
  density: IslandDensity
  docks: Record<string, IslandDock>
}

interface Bounds {
  x: number
  y: number
  width: number
  height: number
}

export interface IslandDisplay {
  id: number
  label: string
  bounds: Bounds
  workArea: Bounds
}

export const defaultIslandControls: IslandControls = {
  placement: 'automatic', displayId: null, locked: true, density: 'compact', docks: {},
}

const edges: readonly IslandEdge[] = ['top', 'bottom', 'left', 'right']
const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))
const finite = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value)
const record = (value: unknown): Record<string, unknown> | null =>
  value !== null && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : null

function normalizedDock(value: unknown): IslandDock | null {
  const dock = record(value)
  if (!dock || !edges.includes(dock.edge as IslandEdge)) return null
  return { edge: dock.edge as IslandEdge, offset: finite(dock.offset) ? clamp(dock.offset, 0, 1) : 0.5 }
}

/** Treat saved preferences as data, including imported or partially written settings. */
export function normalizeIslandControls(input: unknown): IslandControls {
  const value = record(input) ?? {}
  const docks: Record<string, IslandDock> = {}
  const saved = record(value.docks)
  if (saved) {
    for (const [id, candidate] of Object.entries(saved)) {
      // Display IDs are finite integers; this also excludes prototype-related keys.
      if (!/^-?\d+$/.test(id) || !Number.isSafeInteger(Number(id))) continue
      const dock = normalizedDock(candidate)
      if (dock) docks[id] = dock
    }
  }
  return {
    placement: value.placement === 'follow' || value.placement === 'primary' ? value.placement : 'automatic',
    displayId: finite(value.displayId) && Number.isSafeInteger(value.displayId) ? value.displayId : null,
    locked: typeof value.locked === 'boolean' ? value.locked : defaultIslandControls.locked,
    density: value.density === 'expanded' ? 'expanded' : 'compact',
    docks,
  }
}

/** A newly connected or replacement display starts centered at its top edge. */
export function dockFor(controls: IslandControls, displayId: number): IslandDock {
  const id = String(displayId)
  return Object.hasOwn(controls.docks, id) && normalizedDock(controls.docks[id]) || { edge: 'top', offset: 0.5 }
}

/** Keep the transparent window inside the usable area, except for the Mac's centered notch. */
export function islandPanelBounds(display: IslandDisplay, dock: IslandDock, notch: boolean): Bounds {
  const area = display.workArea
  const vertical = dock.edge === 'left' || dock.edge === 'right'
  const width = Math.max(0, Math.min(vertical ? 420 : 720, Math.floor(area.width)))
  const height = Math.max(0, Math.min(vertical ? 560 : 380, Math.floor(area.height)))
  const offset = finite(dock.offset) ? clamp(dock.offset, 0, 1) : 0.5
  let x = vertical ? area.x : area.x + Math.round((area.width - width) * offset)
  let y = vertical ? area.y + Math.round((area.height - height) * offset) : area.y
  if (dock.edge === 'right') x = area.x + area.width - width
  if (dock.edge === 'bottom') y = area.y + area.height - height
  if (notch && dock.edge === 'top' && offset === 0.5) {
    x = clamp(Math.round(display.bounds.x + (display.bounds.width - width) / 2), area.x, area.x + area.width - width)
    y = display.bounds.y
  }
  return { x, y, width, height }
}

/** Release near the closest usable edge; the pointer becomes the panel's center along it. */
export function snapIslandDock(point: { x: number; y: number }, display: IslandDisplay, panel: { width: number; height: number }): IslandDock {
  const area = display.workArea
  const x = finite(point.x) ? point.x : area.x + area.width / 2
  const y = finite(point.y) ? point.y : area.y + area.height / 2
  const distances: Record<IslandEdge, number> = {
    top: Math.abs(y - area.y), bottom: Math.abs(y - area.y - area.height),
    left: Math.abs(x - area.x), right: Math.abs(x - area.x - area.width),
  }
  const edge = edges.reduce((best, next) => distances[next] < distances[best] ? next : best)
  const vertical = edge === 'left' || edge === 'right'
  const span = vertical ? area.height : area.width
  const size = vertical ? panel.height : panel.width
  const position = vertical ? y - area.y : x - area.x
  return { edge, offset: span > size ? clamp((position - size / 2) / (span - size), 0, 1) : 0.5 }
}

/** Match the renderer's edge anchor, including a zero-width side wake strip. */
export function localIslandRect(frame: { width: number; height: number }, panelWidth: number, panelHeight: number, edge: IslandEdge): Bounds {
  const width = clamp(frame.width, 0, panelWidth)
  const height = clamp(frame.height, 0, panelHeight)
  const x = edge === 'left' ? 0 : edge === 'right' ? panelWidth - width : (panelWidth - width) / 2
  const y = edge === 'top' ? 0 : edge === 'bottom' ? panelHeight - height : (panelHeight - height) / 2
  return { x, y, width, height }
}
