import { defaultUsagePreferences, type IslandUsagePreferences } from '@shared/island-activity'
import { pref, usePref } from '../../lib/prefs'

export const islandUsage = pref<IslandUsagePreferences>('islandUsagePreferences', defaultUsagePreferences)

/**
 * The agent dock: a compact roster of this computer's bots in the island, on the island's own edge.
 * Off by default, and a device setting like the rest of the island's options.
 */
export const islandAgentDock = pref('islandAgentDock', false)

export function useIslandAgentDock() {
  return usePref(islandAgentDock)
}

export function useIslandUsage() {
  const [raw, set] = usePref(islandUsage)
  const value: IslandUsagePreferences = {
    providers: Array.isArray(raw?.providers) ? raw.providers.filter((id): id is string => typeof id === 'string') : [],
    windows: Array.isArray(raw?.windows) ? raw.windows.filter((id): id is string => typeof id === 'string') : [],
    notices: raw?.notices === true,
    threshold: Number.isFinite(raw?.threshold) ? Math.min(100, Math.max(50, raw.threshold)) : 90,
  }
  return [value, set] as const
}

export const togglePin = (pins: string[], id: string) => pins.includes(id) ? pins.filter((pin) => pin !== id) : [...pins, id]
