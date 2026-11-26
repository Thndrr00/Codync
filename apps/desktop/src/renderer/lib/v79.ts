//! Pure decisions from OpenBot v0.79.0, shared by the desktop views.

type Draft = { model?: string | null; effort?: string | null } | null

/** Keep the chosen model and reasoning level while the New agent screen is left and reopened. */
export function keepDraft(saved: Draft, current: Draft): Draft {
  if (!saved || !current) return current ?? saved ?? null
  return { ...current, model: current.model || saved.model, effort: current.effort || saved.effort }
}

/** An idle-update notice links to Settings > Updates; a required update must restart at once. */
export function updateNotice(kind: string): string | null {
  return kind === 'idle' ? 'Open update settings' : null
}

/** Internal harness instructions are never shown as words the person spoke. */
export function spokenRequest(text: string): string {
  return text
    .split('\n')
    .filter((line: string) => !/^\s*#\s*AGENTS\.md/i.test(line.trim()))
    .join('\n')
    .trim()
}

/** A short what-to-do hint for a failed workflow step, from its failure kind. */
export function stepErrorHint(kind: string): string {
  if (kind === 'signin') return 'Sign in again in Settings, then run this step.'
  if (kind === 'model') return 'Choose another model for this step.'
  if (kind === 'limit') return 'Wait for the limit to reset, or use a backup model.'
  if (kind === 'field') return 'Give every path into this step the field, or mark it optional.'
  return 'Check the step settings and run it again.'
}

/** Connection hit targets stay usable when the canvas is zoomed out. */
export function portScale(zoom: number): number {
  if (zoom >= 0.5) return 1
  if (zoom >= 0.25) return 2
  return 2
}
