/**
 * The pace mark of a plan window (OpenBot 0.72.0): the share of the limit an even spend would have
 * used by now, from the window's start to its reset. Ported to the desktop's usage bars; the host keeps
 * the same arithmetic in `usage::pace_percent`.
 */

export interface PacedWindow {
  resetsAt?: number | null
  windowSeconds?: number | null
}

/** The share used at an even pace, 0–100, or null when the reset or the length is unknown. */
export function pacePercent(window: PacedWindow, now: number): number | null {
  const resetsAt = window.resetsAt
  const seconds = window.windowSeconds
  if (typeof resetsAt !== 'number' || typeof seconds !== 'number' || !Number.isFinite(resetsAt) || seconds <= 0) return null
  const length = seconds * 1000
  const elapsed = Math.max(0, now - (resetsAt - length))
  return Math.min(100, (elapsed / length) * 100)
}
