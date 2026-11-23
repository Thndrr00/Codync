// Where a long new agent reply is shown (OpenBot nightly triage-12962). A reader following the end
// sees the start of a reply that arrives whole and is taller than the view; everything else keeps
// the old "scroll to the end" behavior. Pure decision, tested without a DOM.

export interface AnswerRow {
  id: string
  kind: 'entry' | 'exchanges' | 'separator'
  author: string
  final: boolean
}

/**
 * The id of a final agent answer appended at the end, or null. A chat that just opened, an answer
 * already seen, history loaded above, a sent message and progress notes are all left alone.
 */
export function newFinalAnswerAtEnd(before: readonly string[] | null, after: readonly AnswerRow[]): string | null {
  if (!before || !after.length) return null
  const last = after[after.length - 1]!
  if (last.kind !== 'entry' || last.author !== 'agent' || !last.final) return null
  if (before.includes(last.id)) return null
  // The reply (and at most a time separator) must be appended after a row we already drew.
  // A history page prepends rows whose last one is old; a switch or a jump has no new last row.
  let anchor = -1
  for (let i = after.length - 1; i >= 0; i -= 1) {
    if (before.includes(after[i]!.id)) { anchor = i; break }
  }
  if (anchor < 0) return null
  for (let i = anchor + 1; i < after.length - 1; i += 1) {
    if (after[i]!.kind !== 'separator') return null
  }
  return last.id
}
