// The composer's quick commands: a message that starts with `/task` or `/schedule` is not sent to
// the bot; it becomes a board card or a scheduled message. Everything else, including other slash
// commands, goes on as usual.

export type QuickCommand =
  | { kind: 'task'; title: string }
  | { kind: 'schedule'; sendAt: number; text: string }
  | { kind: 'error'; message: string }

const UNITS: [string, number][] = [
  ['m', 60_000],
  ['h', 3_600_000],
  ['d', 86_400_000],
]

/** `draft` as a quick command, or null when it is an ordinary message. `now` keeps it testable. */
export function parseQuickCommand(draft: string, now: number): QuickCommand | null {
  const match = /^\/(task|schedule)\b\s*([\s\S]*)$/i.exec(draft.trim())
  if (!match) return null
  const command = match[1]!.toLowerCase()
  const body = match[2]!.trim()
  // `/task` alone is the slash menu filling in the command, not a card with no title.
  if (!body) return null
  if (command === 'task') {
    return { kind: 'task', title: body.slice(0, 200) }
  }
  const when = /^(\d+)\s*([a-z]+)\s+([\s\S]+)$/i.exec(body)
  if (!when) return { kind: 'error', message: 'Use /schedule 30m <message> (m, h or d).' }
  const amount = Number(when[1])
  const unit = UNITS.find(([letter]) => when[2]!.toLowerCase().startsWith(letter))
  const text = when[3]!.trim()
  if (!Number.isFinite(amount) || amount <= 0 || !unit) {
    return { kind: 'error', message: 'Use /schedule 30m <message> (m, h or d).' }
  }
  if (!text) return { kind: 'error', message: 'Write the message after the time.' }
  return { kind: 'schedule', sendAt: now + amount * unit[1], text }
}
