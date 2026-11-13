// A line-by-line comparison for the document history: what a revision had, next to what the editor
// has now. It is a plain longest-common-subsequence walk, capped so a huge document stays fast; above
// the cap the caller shows the two texts instead.

export interface DocDiffLine {
  kind: 'same' | 'added' | 'removed'
  text: string
  oldNumber: number | null
  newNumber: number | null
}

const MAX_LINES = 1_200

export function lineDiff(before: string, after: string): DocDiffLine[] | null {
  const a = before.split('\n')
  const b = after.split('\n')
  if (a.length > MAX_LINES || b.length > MAX_LINES) return null
  const width = b.length + 1
  const dp = new Uint32Array((a.length + 1) * width)
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      dp[i * width + j] = a[i] === b[j]
        ? dp[(i + 1) * width + j + 1]! + 1
        : Math.max(dp[(i + 1) * width + j]!, dp[i * width + j + 1]!)
    }
  }
  const rows: DocDiffLine[] = []
  let i = 0
  let j = 0
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      rows.push({ kind: 'same', text: a[i]!, oldNumber: i + 1, newNumber: j + 1 })
      i++
      j++
    } else if (dp[(i + 1) * width + j]! >= dp[i * width + j + 1]!) {
      rows.push({ kind: 'removed', text: a[i]!, oldNumber: i + 1, newNumber: null })
      i++
    } else {
      rows.push({ kind: 'added', text: b[j]!, oldNumber: null, newNumber: j + 1 })
      j++
    }
  }
  while (i < a.length) rows.push({ kind: 'removed', text: a[i]!, oldNumber: ++i, newNumber: null })
  while (j < b.length) rows.push({ kind: 'added', text: b[j]!, oldNumber: null, newNumber: ++j })
  return rows
}
