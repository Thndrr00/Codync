/**
 * The open workflow tabs of one device. The pref holds ids only — never a workflow name, its steps
 * or anything else a secret could hide in. Every operation here is pure so the tab strip cannot
 * leave a duplicate, a stale id or no tab at all behind.
 */

const MAX_ID = 200
/** Host-generated workflow ids are plain tokens; paths, URLs and control characters are foreign data. */
const unsafeId = /[\u0000-\u001f\u007f/\\@:"'\s]/

export function safeWorkflowId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_ID && !unsafeId.test(value)
}

/** The saved ids, cleaned and deduplicated, with the workflow the view was opened on brought back. */
export function restoreWorkflowTabs(saved: unknown, incomingId: string): string[] {
  const ids: string[] = []
  for (const value of Array.isArray(saved) ? saved : []) {
    if (safeWorkflowId(value) && !ids.includes(value)) ids.push(value)
  }
  if (safeWorkflowId(incomingId) && !ids.includes(incomingId)) ids.push(incomingId)
  return ids
}

/** Opens a workflow: a repeat click brings its existing tab to the end. */
export function openWorkflow(ids: readonly string[], id: string): string[] {
  if (!safeWorkflowId(id)) return [...ids]
  return [...ids.filter((item) => item !== id), id]
}

/** Closes one tab. The last tab stays, so the canvas always has a workflow. */
export function closeWorkflow(ids: readonly string[], id: string): string[] {
  if (ids.length <= 1 || !ids.includes(id)) return [...ids]
  return ids.filter((item) => item !== id)
}

/** The tab to select after `id` closes: its right neighbour, else its left one. */
export function nextActiveTab(ids: readonly string[], id: string, current: string): string | null {
  const at = ids.indexOf(id)
  if (at < 0) return safeWorkflowId(current) && ids.includes(current) ? current : (ids[0] ?? null)
  const remaining = ids.filter((item) => item !== id)
  if (current !== id || remaining.length === 0) return null
  return remaining[Math.min(at, remaining.length - 1)] ?? null
}

/** Leaves only `id` open. */
export function closeOtherWorkflows(ids: readonly string[], id: string): string[] {
  return ids.includes(id) ? [id] : [...ids]
}

/** Closes every tab to the right of `id`, keeping it selected. */
export function closeWorkflowsRight(ids: readonly string[], id: string): string[] {
  const at = ids.indexOf(id)
  return at < 0 ? [...ids] : ids.slice(0, at + 1)
}

/** A held tab dropped on `to` takes its place in the strip. */
export function moveWorkflow(ids: readonly string[], from: string, to: string): string[] {
  if (from === to || !ids.includes(from) || !ids.includes(to)) return [...ids]
  const rest = ids.filter((item) => item !== from)
  rest.splice(rest.indexOf(to), 0, from)
  return rest
}
