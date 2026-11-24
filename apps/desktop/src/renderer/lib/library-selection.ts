/** Which keys are selected in a chat's Library, and the key a Shift-click ranges from. */
export interface LibrarySelection {
  ids: string[]
  anchor: string | null
}

/** The click modifiers the Library uses: Command on macOS, Ctrl elsewhere, and Shift for a range. */
export interface SelectionModifiers {
  meta: boolean
  shift: boolean
}

/**
 * The selection after clicking one item. A plain click picks one; Command-click toggles it; Shift-click
 * takes the range from the anchor in the visible order, even when the click walked past the anchor.
 * Keys outside the visible list are ignored, so filtering the Library cannot keep a hidden selection.
 */
export function selection_after(
  current: readonly string[],
  clicked: string | null,
  anchor: string | null,
  modifiers: SelectionModifiers,
  ids: readonly string[],
): LibrarySelection {
  if (!clicked || !ids.includes(clicked)) return { ids: current.filter((id) => ids.includes(id)), anchor: null }
  if (modifiers.shift) {
    const to = ids.indexOf(clicked)
    const from = anchor ? ids.indexOf(anchor) : -1
    if (from < 0) return { ids: [clicked], anchor: clicked }
    return { ids: ids.slice(Math.min(from, to), Math.max(from, to) + 1), anchor }
  }
  if (modifiers.meta) {
    const next = current.includes(clicked) ? current.filter((id) => id !== clicked) : [...current, clicked]
    return { ids: ids.filter((id) => next.includes(id)), anchor: clicked }
  }
  return { ids: [clicked], anchor: clicked }
}

/** The selection after a marquee drag: replace it, or add the hit items when Command/Ctrl was held. */
export function marquee_after(current: readonly string[], hits: readonly string[], additive: boolean): string[] {
  const next = additive ? [...current, ...hits] : [...hits]
  return [...new Set(next)]
}
