// Edge affordances for wide Markdown tables and long code lines. Pure measurements so the arrow
// decisions can be tested without a DOM; HScroll owns the actual element and its scroll position.

export interface ScrollEdges {
  left: boolean
  right: boolean
}

/** Which edges still have content beyond them (a one-pixel tolerance for layout rounding). */
export function hscrollEdges(scrollLeft: number, clientWidth: number, scrollWidth: number, epsilon = 1): ScrollEdges {
  const farthest = Math.max(0, scrollWidth - clientWidth)
  return {
    left: farthest > 0 && scrollLeft > epsilon,
    right: farthest > 0 && scrollLeft < farthest - epsilon,
  }
}

/** One arrow press moves almost a viewport, so a wide table crosses in a few taps. */
export function hscrollStep(clientWidth: number): number {
  return Math.max(120, clientWidth * 0.8)
}
