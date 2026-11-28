/**
 * In the collapsed sidebar the popover names the agent, channel or pinned chat in full on one
 * line; only the Title badge is bounded. A long name is never cut, so it stays identifiable.
 */
const MAX_TITLE = 40

export type PopoverName = { name: string; title: string }

export function popover_name(name: string, title: string): PopoverName {
  const full = (name ?? '').trim()
  const badge = (title ?? '').trim()
  return {
    name: full,
    title: badge.length > MAX_TITLE ? `${badge.slice(0, MAX_TITLE - 1).trimEnd()}…` : badge,
  }
}
