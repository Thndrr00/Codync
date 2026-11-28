import type { Entry } from '../../../shared/models.ts'

/** A bot's entries grouped by turn, oldest turn first, each turn's entries in their order. */
export function traceTurns(entries: Entry[]): [number, Entry[]][] {
  const turns = new Map<number, Entry[]>()
  for (const e of entries) turns.set(e.turn, [...(turns.get(e.turn) ?? []), e])
  return [...turns.entries()].sort((a, b) => a[0] - b[0])
}

/** The turn running now while the bot works in this chat; a queued message already holds the next turn. */
export function runningTurn(entries: Entry[], working: boolean): number | undefined {
  if (!working) return undefined
  return entries.reduce((t, e) => (e.data.status === 'queued' ? t : Math.max(t, e.turn)), -1)
}

/** What the running turn has done so far, in order: the steps the working indicator unfolds. */
export function workingSteps(entries: Entry[]): Entry[] {
  const turn = runningTurn(entries, true)
  return entries.filter((e) => e.turn === turn && isWorkingStep(e))
}

/** Thinking, narration, a tool call or a plan; never the bot's bookkeeping, nor a call the host didn't flag. */
function isWorkingStep(e: Entry): boolean {
  switch (e.kind) {
    case 'thought':
      return !!e.data.text?.trim()
    case 'agent':
      return !e.data.final && !isPass(e.data.text)
    case 'tool':
      return e.data.bookkeeping === false
    case 'plan':
      return true
    default:
      return false
  }
}

/** No text, or a pass the host keeps out of the chat (`is_pass` in host/src/chat/group.rs). */
function isPass(text = ''): boolean {
  const t = text.trim().replace(/^[".*]+|[".*]+$/g, '').toLowerCase()
  return t === '' || t === '(pass)' || t === 'pass'
}

/** A trace row, or a run of tool steps folded into one "worked on N steps" group. */
export type TraceSlot = { kind: 'row'; entry: Entry } | { kind: 'work'; entries: Entry[] }

/**
 * Folds a run of at least `min` consecutive tool steps into one slot. Other steps and a shorter
 * run stay as rows, so a single tool call is never hidden behind a group.
 */
export function groupWorkSteps(entries: Entry[], min = 3): TraceSlot[] {
  const out: TraceSlot[] = []
  let run: Entry[] = []
  const flush = () => {
    if (run.length >= min) out.push({ kind: 'work', entries: run })
    else out.push(...run.map((entry) => ({ kind: 'row' as const, entry })))
    run = []
  }
  for (const entry of entries) {
    if (entry.kind === 'tool') run.push(entry)
    else {
      flush()
      out.push({ kind: 'row', entry })
    }
  }
  flush()
  return out
}

/** Scrolled to (within a few pixels of) the end: new steps keep it there. */
export function scrolledToEnd(el: { scrollHeight: number; scrollTop: number; clientHeight: number }) {
  return el.scrollHeight - el.scrollTop - el.clientHeight < 8
}
