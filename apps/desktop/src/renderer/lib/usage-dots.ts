/** Reports can arrive outside a limit's range; zero must stay visually empty. */
export const usagePercent = (percent: number) => Number.isFinite(percent) ? Math.min(100, Math.max(0, percent)) : 0

/** Round dots on a four-point grid, filled by whole columns from the left. */
export function usageDotGrid(width: number, height: number, percent: number) {
  const columns = Math.max(1, Math.floor(width / 4))
  const rows = Math.max(1, Math.floor(height / 4))
  const stepX = width / columns
  const stepY = height / rows
  return { columns, rows, stepX, stepY, radius: Math.max(0, Math.min(2.5, stepX, stepY) / 2), filled: Math.round(columns * usagePercent(percent) / 100) }
}
