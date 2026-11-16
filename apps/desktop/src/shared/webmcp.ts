// WebMCP (card K15): the declared actions a mini-app page served by the host may ask for. The host
// owns the list and the permission; this file only names them for the app panel.

export interface WebMcpAction {
  name: string
  description: string
  granted: boolean
}

/** The whole allow list, so a host that knows a newer action still shows its own words. */
export const WEB_MCP_ACTIONS = ['open_chat', 'show_artifact', 'start_call'] as const

const LABELS: Record<string, string> = {
  open_chat: 'Open a bot chat',
  show_artifact: 'Show an artifact',
  start_call: 'Start a call',
}

export const webMcpLabel = (name: string) => LABELS[name] ?? name

/** The granted names, in the declared order, which is what the host stores. */
export function grantedActions(actions: readonly WebMcpAction[]): string[] {
  return actions.filter((action) => action.granted).map((action) => action.name)
}

/** The same list with one action turned on or off. */
export function toggledAction(actions: readonly WebMcpAction[], name: string, granted: boolean): WebMcpAction[] {
  return actions.map((action) => (action.name === name ? { ...action, granted } : action))
}

/** Fills a host answer whose grants the page might have learned after the panel opened. */
export function normalizeActions(value: unknown): WebMcpAction[] {
  if (!Array.isArray(value)) return []
  return value
    .filter((action): action is Record<string, unknown> => !!action && typeof action === 'object')
    .map((action) => ({
      name: String(action.name ?? ''),
      description: String(action.description ?? ''),
      granted: action.granted === true,
    }))
    .filter((action) => action.name !== '' && WEB_MCP_ACTIONS.includes(action.name as typeof WEB_MCP_ACTIONS[number]))
}
