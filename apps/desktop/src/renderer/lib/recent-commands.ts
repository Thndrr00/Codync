/** The command palette remembers the last few picks on this device, newest first. */
export const RECENT_COMMAND_LIMIT = 6

/**
 * Move `id` to the front once and keep at most `limit` ids. A pure reducer for the device pref, so a
 * repeat pick never takes two slots and the list stays bounded however often the palette is used.
 */
export function pushRecentCommand(ids: readonly string[], id: string, limit = RECENT_COMMAND_LIMIT): string[] {
  const command = id.trim()
  if (!command || limit <= 0) return [...ids]
  return [command, ...ids.filter((value) => value !== command)].slice(0, limit)
}
