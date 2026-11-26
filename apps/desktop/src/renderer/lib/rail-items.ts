/** The rail's optional pages. Credits stays in the rail, so it is never a hide/show choice. */
export const RAIL_ITEMS = ['marketplace', 'help', 'credits'] as const

export type RailItem = typeof RAIL_ITEMS[number]

/**
 * Pure decision: which optional rail pages remain visible. A hidden Credits is ignored — the rail
 * always keeps it — and unknown ids in a saved preference can't hide a page or invent one.
 */
export function railItems(hidden: readonly string[] | null | undefined): RailItem[] {
  return RAIL_ITEMS.filter((id) => id === 'credits' || !hidden?.includes(id))
}
