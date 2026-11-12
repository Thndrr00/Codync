/** Device presentation of host state; no credentials or full message bodies cross to the island. */
export interface IslandRun {
  id: string
  ref: string
  name: string
  status: string
  detail: string
  at: number
  routineId?: string
  startedAt?: number | null
  finishedAt?: number | null
  retryable?: boolean
  steps: { name: string; status: string }[]
}

export interface IslandChannelEvent {
  id: string
  ref: string
  source: string
  channel: string
  at: number
}

export interface IslandCall { id: string; ref: string; name: string; phase: string; muted: boolean }

export interface IslandActivity {
  online: boolean
  connection: string
  route: string
  refreshing: boolean
  refreshError: string | null
  error: string | null
  updatedAt: number | null
  health: { label: string; value: string }[]
  runs: IslandRun[]
  unread: { ref: string; name: string; count: number }[]
  channels: IslandChannelEvent[]
  call: IslandCall | null
  n8n: { state: string; error: string | null; mcp: boolean } | null
}

export type IslandAction =
  | { kind: 'refreshUsage' }
  | { kind: 'refreshActivity' }
  | { kind: 'stopBot'; ref: string }
  | { kind: 'openFlow'; ref: string }
  | { kind: 'openChannels' }
  | { kind: 'openDiagnostics' }
  | { kind: 'callMute'; callId: string }
  | { kind: 'callEnd'; callId: string }
  | { kind: 'loadWorkspace'; ref: string }
  | { kind: 'resumeDraft'; ref: string }
  | { kind: 'sendSavedDraft'; ref: string; text: string; original: string }
  | { kind: 'retryRun'; ref: string; id: string }
  | { kind: 'cancelQueued'; ref: string; id: string }
  | { kind: 'openArtifact'; ref: string; id: string }
  | { kind: 'openFile'; ref: string; id: string; entryId: string }

export interface IslandUsagePreferences {
  providers: string[]
  windows: string[]
  notices: boolean
  threshold: number
}

export const defaultUsagePreferences: IslandUsagePreferences = { providers: [], windows: [], notices: false, threshold: 90 }

/** Pins retain every provider/window, and do not mutate the host's report order. */
export function pinnedFirst<T>(items: readonly T[], pins: readonly string[], id: (item: T) => string): T[] {
  const rank = (item: T) => { const at = pins.indexOf(id(item)); return at < 0 ? pins.length : at }
  return [...items].sort((a, b) => rank(a) - rank(b))
}

export function usageNotice(providers: { id: string; name: string; windows: { id: string; percent: number; text?: string | null; resetsAt?: number | null }[] }[], prefs: IslandUsagePreferences, now: number) {
  if (!prefs.notices) return null
  const limits = providers.flatMap((p) => p.windows.filter((w) => !w.text && Number.isFinite(w.percent)).map((w) => ({ provider: p.name, ...w })))
  const highest = limits.filter((w) => w.percent >= prefs.threshold).sort((a, b) => b.percent - a.percent)[0]
  if (highest) return { kind: 'limit' as const, provider: highest.provider, percent: highest.percent, resetsAt: highest.resetsAt ?? null }
  const reset = limits.filter((w) => w.percent > 0 && w.resetsAt && w.resetsAt > now && w.resetsAt - now <= 15 * 60_000).sort((a, b) => a.resetsAt! - b.resetsAt!)[0]
  return reset ? { kind: 'reset' as const, provider: reset.provider, percent: reset.percent, resetsAt: reset.resetsAt! } : null
}
