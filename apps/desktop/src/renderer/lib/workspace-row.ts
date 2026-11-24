/** The top row of a bot's Files sheet: the bot's own workspace, never a path from its file list. */
export interface WorkspaceRow {
  /** The absolute folder path main is allowed to reveal. */
  path: string
  /** The folder's name, or the workspace label when the path has none. */
  label: string
}

const WORKSPACE_FALLBACK_LABEL = 'Workspace'

/** Validates `bot.cwd` and names the row. Rejects relative paths and traversal before any IPC call. */
export function workspace_row(cwd: unknown): WorkspaceRow | null {
  if (typeof cwd !== 'string') return null
  const trimmed = cwd.trim()
  if (!trimmed || /[\0\r\n]/.test(trimmed)) return null
  const absolute = /^(?:[A-Za-z]:[\\/]|[\\/]{2}|[\\/])/.test(trimmed)
  if (!absolute || trimmed.split(/[\\/]/).includes('..')) return null
  const stripped = trimmed.replace(/[\\/]+$/, '')
  const path = /^[A-Za-z]:$/.test(stripped) ? trimmed : stripped
  const parts = path.split(/[\\/]/).filter(Boolean)
  const root = parts.length === 0 || (parts.length === 1 && /^[A-Za-z]:$/.test(parts[0] ?? ''))
  const label = root ? '' : parts[parts.length - 1]?.replace(/^\s+|\s+$/g, '') ?? ''
  return { path, label: label || WORKSPACE_FALLBACK_LABEL }
}
