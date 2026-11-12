/** Operational metadata only. Commands, capabilities, pairing codes and content remain with their owners. */
export interface IslandRuntime {
  context: { used: number | null; size: number | null; lastTurn: number | null; compactions: number } | null
  cost: { amount: null; currency: null; reason: string }
  mcp: { id: string; connectorId: string; tool: string; status: string; createdAt: number }[]
  error: string | null
}
export interface IslandGit {
  root: string | null
  conflicts: string[]
  worktrees: { path: string; branch: string | null; head: string | null; detached: boolean; locked: boolean; prunable: boolean; botIds: string[] }[]
  error: string | null
}
export interface IslandResources {
  documents: { id: string; title: string; updatedAt: number }[]
  channelFailures: { id: string; source: string; state: string }[]
  channelState: { source: string; state: string }[]
  scope: 'computer'
  error: string | null
}
export interface IslandOperations {
  ref: string
  loading: boolean
  actionError: string | null
  errors: { section: string; message: string }[]
  at: number | null
  runtime: IslandRuntime | null
  usage: { used: number; limit: number | null; fleetUsed: number; fleetLimit: number | null; resetsAt: number } | null
  budgets: { botId: string; limit: number; used: number; level: string }[]
  health: { cpu: number; ramUsedPercent: number; ramFree: number; ramTotal: number; diskFree: number; diskTotal: number; at: number; rss: number | null } | null
  fallback: { agent: string; model: string | null; reason: string | null; since: number } | null
  compactions: { id: string; at: number; byHost: boolean }[]
  switches: { id: string; at: number; from: string; to: string; reason: string | null; back: boolean }[]
  resources: IslandResources | null
  git: IslandGit | null
  apps: { slug: string; title: string; version: number; updatedAt: number }[]
  computer: { enabled: boolean; userControl: boolean; agentBot: string | null; computerUse: boolean } | null
  hostUpdate: { currentVersion: string; phase: string; availableVersion: string | null; error: string | null } | null
  localComputer: boolean
  browserOpen: boolean | null
}
export interface IslandOperationSignals {
  updateError: string | null
  updates: { supported: boolean; checking: boolean; availableVersion: string | null; downloadPercent: number | null; staged: boolean; installing: boolean; error: string | null } | null
  ssh: { id: string; name: string; kind: string; detail: string | null; retryable: boolean }[]
  devices: { id: string; computer: string; name: string; platform: string | null; kind: 'access' | 'pairing' }[]
}
export type IslandOperationsAction =
  | { kind: 'loadOperations'; ref: string }
  | { kind: 'openOperationPanel'; ref: string; panel: 'trace' | 'computer' | 'budget' }
  | { kind: 'openOperationDocument'; ref: string; id: string }
  | { kind: 'openOperationApp'; ref: string; id: string }
  | { kind: 'selectWorktreeBot'; ref: string; id: string }
  | { kind: 'openExistingBrowser'; ref: string }
  | { kind: 'retryIslandSSH'; id: string }
  | { kind: 'reviewDevices' }
  | { kind: 'openOperationSettings'; page: 'updates' | 'computers' | 'channels' }

export function islandContextPercent(context: IslandRuntime['context']): number | null {
  return context && Number.isFinite(context.used) && Number.isFinite(context.size) && context.used !== null && context.size !== null && context.used >= 0 && context.size > 0 ? Math.min(100, Math.round(context.used / context.size * 100)) : null
}
export const islandAppFavoriteKey = (ref: string, slug: string) => JSON.stringify([ref, slug])
