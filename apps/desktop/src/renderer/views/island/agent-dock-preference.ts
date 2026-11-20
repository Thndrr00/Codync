import { useSyncExternalStore } from 'react'

// The agent dock is device UI state: where it sits, how big it is and which bots have a seat.
// Bots, work and messages stay with the host; this file only holds what this computer chose.

export type AgentDockEdge = 'left' | 'right'

export interface AgentDockPreference {
  shown: boolean
  edge: AgentDockEdge
  /** The dock's height as a percent of the island window's height. */
  sizePercent: number
  autoHide: boolean
  /** Bot refs in seat order; empty means "use the roster default" (see `dockSeats`). */
  seats: string[]
}

export const AGENT_DOCK_MAX_SEATS = 5
/** Until the person saves an order, the dock gives the first few visible roster bots a seat. */
export const AGENT_DOCK_DEFAULT_SEATS = 3
export const AGENT_DOCK_SIZE = { min: 20, max: 70, default: 40 } as const

export const defaultAgentDockPreference: AgentDockPreference = {
  shown: false,
  edge: 'right',
  sizePercent: AGENT_DOCK_SIZE.default,
  autoHide: true,
  seats: [],
}

const KEY = 'islandAgentDockPreference'
const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)

/** Treat saved values as data: a partial or hand-edited preference falls back field by field. */
export function normalizeAgentDockPreference(input: unknown): AgentDockPreference {
  const value = record(input) ? input : {}
  const size = typeof value.sizePercent === 'number' && Number.isFinite(value.sizePercent)
    ? Math.round(Math.min(AGENT_DOCK_SIZE.max, Math.max(AGENT_DOCK_SIZE.min, value.sizePercent)))
    : defaultAgentDockPreference.sizePercent
  const seats = Array.isArray(value.seats)
    ? [...new Set(value.seats
      .filter((seat): seat is string => typeof seat === 'string' && seat.trim().length > 0)
      .map((seat) => seat.trim()))].slice(0, AGENT_DOCK_MAX_SEATS)
    : []
  return {
    shown: value.shown === true,
    edge: value.edge === 'left' ? 'left' : 'right',
    sizePercent: size,
    autoHide: typeof value.autoHide === 'boolean' ? value.autoHide : defaultAgentDockPreference.autoHide,
    seats,
  }
}

function read(): AgentDockPreference {
  try {
    return normalizeAgentDockPreference(JSON.parse(localStorage.getItem(KEY) ?? 'null'))
  } catch {
    return defaultAgentDockPreference
  }
}

let current = read()
const listeners = new Set<() => void>()

if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key !== null && event.key !== KEY) return
    current = read()
    for (const listener of listeners) listener()
  })
}

export const agentDockPreference = {
  get: () => current,
  set(patch: Partial<AgentDockPreference>) {
    current = normalizeAgentDockPreference({ ...current, ...patch })
    try {
      localStorage.setItem(KEY, JSON.stringify(current))
    } catch {
      // A full store keeps the value in memory for this session.
    }
    for (const listener of listeners) listener()
  },
  subscribe(listener: () => void) {
    listeners.add(listener)
    return () => void listeners.delete(listener)
  },
}

export function useAgentDockPreference() {
  return [useSyncExternalStore(agentDockPreference.subscribe, agentDockPreference.get), agentDockPreference.set] as const
}

/**
 * The seats to draw: the saved order (without refs that left the roster), or the first few visible
 * roster bots before the person has saved one. Bots found by both rules are never duplicated.
 */
export function dockSeats(preference: AgentDockPreference, botRefs: readonly string[]): string[] {
  const visible: string[] = []
  for (const ref of botRefs) if (!visible.includes(ref)) visible.push(ref)
  const saved = preference.seats.filter((ref) => visible.includes(ref))
  return (saved.length ? saved : visible.slice(0, AGENT_DOCK_DEFAULT_SEATS)).slice(0, AGENT_DOCK_MAX_SEATS)
}
