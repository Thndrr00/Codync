// A unified diff split into rows for display (idea 26, as Android's DiffRows). Viewing never
// applies the patch. Line numbers appear only where a hunk's `@@ -a,b +c,d @@` ranges let them be
// counted; a diff that does not parse (or is huge) returns null so the caller shows the original.

export type DiffRow =
  | { kind: 'header'; raw: string }
  | { kind: 'hunk'; raw: string }
  | { kind: 'note'; raw: string }
  | { kind: 'context'; raw: string; oldNumber: number | null; newNumber: number | null }
  | { kind: 'added'; raw: string; newNumber: number | null }
  | { kind: 'removed'; raw: string; oldNumber: number | null }

export const MAX_DIFF_LINES = 2_000

const hunkPattern = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/

const headerPrefixes = [
  'diff ', 'index ', '--- ', '+++ ', 'new file', 'deleted file', 'old mode', 'new mode',
  'similarity index', 'dissimilarity index', 'rename ', 'copy ', 'Binary files', 'GIT binary patch',
]

/** Whether a fenced block is, or looks like, a unified diff. */
export function looksLikeDiff(language: string, text: string): boolean {
  const tag = language.trim().toLowerCase()
  if (tag === 'diff' || tag === 'patch') return true
  if (tag !== '') return false
  const first = text.split('\n').find((l) => l.trim() !== '')
  return first !== undefined && (first.startsWith('diff --git ') || hunkPattern.test(first))
}

export function parseDiff(text: string): DiffRow[] | null {
  const lines = text.split('\n').map((l) => l.endsWith('\r') ? l.slice(0, -1) : l)
  if (lines.length > MAX_DIFF_LINES) return null
  const rows: DiffRow[] = []
  let oldLeft = 0
  let newLeft = 0
  let old = 0
  let next = 0
  let recognised = false
  for (const line of lines) {
    const inHunk = oldLeft > 0 || newLeft > 0
    const hunk = inHunk ? null : hunkPattern.exec(line)
    if (hunk) {
      const a = Number(hunk[1]!)
      const b = hunk[2] === undefined ? 1 : Number(hunk[2])
      const c = Number(hunk[3]!)
      const d = hunk[4] === undefined ? 1 : Number(hunk[4])
      old = a; oldLeft = b
      next = c; newLeft = d
      rows.push({ kind: 'hunk', raw: line }); recognised = true
    } else if (line.startsWith('\\')) {
      rows.push({ kind: 'note', raw: line })
    } else if (!inHunk && headerPrefixes.some((p) => line.startsWith(p))) {
      rows.push({ kind: 'header', raw: line }); recognised = true
    } else if (line.startsWith('+')) {
      rows.push({ kind: 'added', raw: line, newNumber: newLeft > 0 ? next : null })
      if (newLeft > 0) { next++; newLeft-- }
    } else if (line.startsWith('-')) {
      rows.push({ kind: 'removed', raw: line, oldNumber: oldLeft > 0 ? old : null })
      if (oldLeft > 0) { old++; oldLeft-- }
    } else if (line.startsWith(' ') || (line === '' && inHunk)) {
      rows.push({ kind: 'context', raw: line, oldNumber: oldLeft > 0 ? old : null, newNumber: newLeft > 0 ? next : null })
      if (oldLeft > 0) { old++; oldLeft-- }
      if (newLeft > 0) { next++; newLeft-- }
    } else if (line === '') {
      rows.push({ kind: 'note', raw: line })
    } else {
      return null
    }
  }
  // A trailing newline leaves one empty line that is not part of the patch.
  if (rows.length > 0 && rows[rows.length - 1]!.kind === 'note' && rows[rows.length - 1]!.raw === '') rows.pop()
  return recognised ? rows : null
}
