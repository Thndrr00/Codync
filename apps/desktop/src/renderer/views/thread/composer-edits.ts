// Composer shortcuts as pure edits (idea 27, as Android's ComposerEdits). Each returns one
// replacement of a range, never the whole draft, so the field keeps its undo history and the
// caller only moves the selection. No shortcut sends anything.

export interface ComposerEdit {
  start: number
  end: number
  replacement: string
  selectionStart: number
  selectionEnd: number
}

const FENCE = '```'

/**
 * Wraps the selection in code: a single line becomes inline code, several lines a fenced block.
 * With nothing selected, inserts an empty pair and puts the cursor between. A selection that
 * already holds a fence is left alone (null), since it would end the new block early.
 */
export function wrapInCode(text: string, selStart: number, selEnd: number): ComposerEdit | null {
  const [start, end] = ordered(text, selStart, selEnd)
  const selected = text.slice(start, end)
  if (selected.includes(FENCE)) return null
  return selected === ''
    ? inlineCode('', start, end)
    : selected.includes('\n') ? fenced(text, selected, start, end) : inlineCode(selected, start, end)
}

/** Inserts a fenced block on its own lines, around the selection when there is one. */
export function insertCodeBlock(text: string, selStart: number, selEnd: number): ComposerEdit | null {
  const [start, end] = ordered(text, selStart, selEnd)
  const selected = text.slice(start, end)
  return selected.includes(FENCE) ? null : fenced(text, selected, start, end)
}

/** Prefixes every line of the selection (or the cursor's line) with a quote marker. */
export function quote(text: string, selStart: number, selEnd: number): ComposerEdit {
  const [start, end] = ordered(text, selStart, selEnd)
  const lineStart = text.lastIndexOf('\n', start - 1) + 1
  const at = text.indexOf('\n', end)
  const lineEnd = at < 0 ? text.length : at
  const quoted = text.slice(lineStart, lineEnd).split('\n').map((l) => `> ${l}`).join('\n')
  return { start: lineStart, end: lineEnd, replacement: quoted, selectionStart: lineStart, selectionEnd: lineStart + quoted.length }
}

/** Formats a whole message's text as a multi-line quote to insert into the draft (idea 34's fallback). */
export function quoteText(text: string): string {
  return text.split('\n').map((l) => `> ${l}`).join('\n')
}

function inlineCode(selected: string, start: number, end: number): ComposerEdit {
  const replacement = `\`${selected}\``
  return selected === ''
    ? { start, end, replacement, selectionStart: start + 1, selectionEnd: start + 1 }
    : { start, end, replacement, selectionStart: start + 1, selectionEnd: start + 1 + selected.length }
}

function fenced(text: string, selected: string, start: number, end: number): ComposerEdit {
  const before = start > 0 && text[start - 1] !== '\n' ? '\n' : ''
  const after = end < text.length && text[end] !== '\n' ? '\n' : ''
  const open = `${before}${FENCE}\n`
  const replacement = `${open}${selected}\n${FENCE}${after}`
  const bodyStart = start + open.length
  return selected === ''
    ? { start, end, replacement, selectionStart: bodyStart, selectionEnd: bodyStart }
    : { start, end, replacement, selectionStart: bodyStart, selectionEnd: bodyStart + selected.length }
}

/** The selection as `start <= end`, clamped to the text; a reversed drag is the same range. */
function ordered(text: string, a: number, b: number): [number, number] {
  const lo = Math.max(0, Math.min(a, b))
  const hi = Math.max(0, Math.max(a, b))
  return [Math.min(lo, text.length), Math.min(hi, text.length)]
}
