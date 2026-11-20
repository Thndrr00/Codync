/** The setup steps, in the order they help most. All of them are optional. */
export const SETUP_STEP_IDS = ['host', 'access', 'bot', 'message', 'account'] as const
export type SetupStepId = typeof SETUP_STEP_IDS[number]

/** The visible steps and how far along they are, after dismissed ones are taken out. */
export interface SetupProgress {
  visible: SetupStepId[]
  done: number
  total: number
  fraction: number
  complete: boolean
}

/**
 * Progress is derived, never saved: done steps come from live state, dismissed ones from the
 * device pref. Dismissing a step removes it from the total rather than counting it as done, so
 * the bar still means "how much of what you left is set up". A checklist with nothing left
 * counts as complete, instead of showing an empty bar.
 */
export function setupProgress(completed: Record<SetupStepId, boolean>, dismissed: readonly string[]): SetupProgress {
  const visible = SETUP_STEP_IDS.filter((id) => !dismissed.includes(id))
  const done = visible.filter((id) => completed[id]).length
  const total = visible.length
  return {
    visible,
    done,
    total,
    fraction: total === 0 ? 1 : done / total,
    complete: total === 0 || done === total,
  }
}
