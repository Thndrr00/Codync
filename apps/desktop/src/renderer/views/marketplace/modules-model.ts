import type { HostClient } from '../../client/host-client'

// Modules (host/src/modules/): a signed folder the host runs in its own process. Its manifest names the tools a
// bot may call, the events it may hear, the hosts it may reach and how much of a call it may cost.

/** What a module asks for, and what the owner approved of it. */
export interface ModulePermissions {
  tools: string[]
  events: string[]
  network: string[]
  /** `none`, `read` or `write`: how much of a bot's folder it may touch. */
  files: string
}

export interface HostModule {
  id: string
  name: string
  version: string | null
  description: string
  symbol: string | null
  contract: number | null
  enabled: boolean
  /** Why it can't run, when its folder no longer verifies. */
  broken: string | null
  /** `running`, `stopped`, `disabled` or `broken`. */
  state: string
  /** Why its process stopped, when it did. */
  stopped: string | null
  permissions: ModulePermissions | null
  approved: ModulePermissions
  limits: { callsPerMinute: number; timeMs: number; memoryMb: number }
  views: { id: string; title: string }[]
  tools: { name: string; description: string }[]
  installedAt: number
  folder: string
}

/** A module's screen, as rows the apps draw with their own components. */
export interface ModuleView {
  title: string
  sections: { title: string; rows: { label: string; value: string }[] }[]
}

const s = (seconds: number) => seconds * 1000

export const modules = {
  list: async (c: HostClient) => (await c.call<{ modules: HostModule[] }>('modules')).modules,
  /** From a folder on this computer, or from a bundle another one exported. */
  install: async (c: HostClient, source: { folder?: string; bundle?: unknown }) =>
    (await c.call<{ modules: HostModule[] }>('installModule', source, s(60))).modules,
  update: async (c: HostClient, id: string, source: { folder?: string; bundle?: unknown }) =>
    (await c.call<{ modules: HostModule[] }>('updateModule', { id, ...source }, s(60))).modules,
  uninstall: async (c: HostClient, id: string) => (await c.call<{ modules: HostModule[] }>('uninstallModule', { id }, s(30))).modules,
  setEnabled: async (c: HostClient, id: string, enabled: boolean) =>
    (await c.call<{ modules: HostModule[] }>('setModule', { id, enabled }, s(30))).modules,
  approve: async (c: HostClient, id: string, permissions: ModulePermissions | null) =>
    (await c.call<{ modules: HostModule[] }>('approveModule', { id, permissions }, s(30))).modules,
  /** The module as one file for another computer. */
  export: (c: HostClient, id: string) => c.call<{ name: string; bundle: unknown }>('exportModule', { id }, s(30)),
  view: async (c: HostClient, id: string, view: string) =>
    (await c.call<{ view: ModuleView }>('moduleView', { id, view }, s(120))).view,
}

/** The line under a module's name: its state in the words the apps use. */
export function stateText(m: HostModule): string {
  if (m.broken) return 'Broken'
  if (!m.enabled) return 'Off'
  return m.state === 'running' ? 'Running' : 'Ready'
}

/** What the owner approved, in one sentence. */
export function approvedText(m: HostModule): string {
  const approved = m.approved
  if (approved.tools.length === 0 && approved.events.length === 0 && approved.files === 'none') return 'Nothing approved yet'
  const parts = [
    approved.tools.length ? `${approved.tools.length} tools` : null,
    approved.events.length ? `${approved.events.length} events` : null,
    approved.files === 'none' ? null : `${approved.files} files`,
  ].filter(Boolean)
  return parts.join(' · ')
}
