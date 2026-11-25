/**
 * Where a background agent acted, in the coordinates of the display overlay (OpenBot
 * computer-use.md). The agent's event is in global screen pixels; the overlay covers one display.
 * Pure so the bound, the display test and the click flag are tested without Electron.
 */

export type AgentCursorEvent = { kind: 'move' | 'click'; x: number; y: number }
export type AgentCursorDisplay = { x: number; y: number; width: number; height: number }
export type AgentCursorPoint = { x: number; y: number; click: boolean }

/** The furthest an event may be from the screen origin, matching the host's input bound. */
export const CURSOR_COORD_LIMIT = 100_000

/**
 * Turns one agent move/click and a display into an overlay point, or `null` when the event is not a
 * bounded point on that display. A click is marked so the overlay can hold its dot a little longer.
 */
export function cursor_point(
  event: AgentCursorEvent | null | undefined,
  display: AgentCursorDisplay | null | undefined,
): AgentCursorPoint | null {
  if (!event || !display) return null
  if (event.kind !== 'move' && event.kind !== 'click') return null
  const numbers = [event.x, event.y, display.x, display.y, display.width, display.height]
  if (!numbers.every(Number.isFinite)) return null
  if (Math.abs(event.x) > CURSOR_COORD_LIMIT || Math.abs(event.y) > CURSOR_COORD_LIMIT) return null
  if (!(display.width > 0) || !(display.height > 0)) return null
  if (
    event.x < display.x ||
    event.y < display.y ||
    event.x > display.x + display.width ||
    event.y > display.y + display.height
  ) {
    return null
  }
  return {
    x: Math.round(event.x - display.x),
    y: Math.round(event.y - display.y),
    click: event.kind === 'click',
  }
}
