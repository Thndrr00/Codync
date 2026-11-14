// The development Inspector's model (card K15): a bounded, redacted record of host events. Nothing here
// runs in a release build — the renderer only records and mounts the overlay behind `import.meta.env.DEV`.

/** The keys that must never reach the panel, whatever their value. */
const SECRET_KEYS = /(token|secret|password|passwd|authorization|cookie|credential|api[-_]?key|private[-_]?key)/i
/** Free text and prompts are the person's words; the Inspector shows shape, not content. */
const CONTENT_KEYS = /^(text|body|prompt|content|output|input|message|messages|transcript|stack|command|args|cwd|path|dir)$/i
const MAX_STRING = 120
const MAX_ITEMS = 20
const MAX_DEPTH = 4

export interface InspectorEvent {
  at: number
  type: string
  data: Record<string, unknown>
}

/** A URL without its query or fragment: the page can be named, its capability token can't. */
function safeUrl(text: string): string {
  try {
    const url = new URL(text)
    return `${url.origin}${url.pathname}`
  } catch {
    return text
  }
}

function redactString(text: string, key: string): string {
  if (SECRET_KEYS.test(key)) return '[redacted]'
  if (/url|href|link/i.test(key)) return safeUrl(text)
  return text.length > MAX_STRING ? `${text.slice(0, MAX_STRING)}…` : text
}

/** A value the panel may show: no secrets, no prompt text, bounded size and depth. */
export function redactValue(value: unknown, key = '', depth = 0): unknown {
  if (value === null || typeof value === 'boolean' || typeof value === 'number') return value
  if (typeof value === 'string') return redactString(value, key)
  if (depth >= MAX_DEPTH) return '[…]'
  if (Array.isArray(value)) {
    const kept = value.slice(0, MAX_ITEMS).map((item) => redactValue(item, key, depth + 1))
    return value.length > MAX_ITEMS ? [...kept, `+${value.length - MAX_ITEMS}`] : kept
  }
  if (typeof value === 'object') {
    const out: Record<string, unknown> = {}
    for (const [name, item] of Object.entries(value as Record<string, unknown>)) {
      if (SECRET_KEYS.test(name)) {
        out[name] = '[redacted]'
      } else if (CONTENT_KEYS.test(name)) {
        out[name] = '[redacted]'
      } else {
        out[name] = redactValue(item, name, depth + 1)
      }
    }
    return out
  }
  return String(value)
}

/** One host event as the Inspector lists it: its type and a redacted copy of its fields. */
export function redactEvent(event: unknown, at = Date.now()): InspectorEvent {
  const source = event && typeof event === 'object' ? (event as Record<string, unknown>) : {}
  const data = { ...source }
  delete data.type
  return {
    at,
    type: typeof source.type === 'string' ? source.type : 'unknown',
    data: redactValue(data) as Record<string, unknown>,
  }
}

/** The newest events, bounded, so an Inspector left open can't grow without limit. */
export class InspectorLog {
  private entries: InspectorEvent[] = []
  constructor(private readonly limit = 200) {}

  add(event: unknown, at = Date.now()) {
    const entry = redactEvent(event, at)
    this.entries = [...this.entries.slice(-(this.limit - 1)), entry]
    return entry
  }

  get all(): readonly InspectorEvent[] {
    return this.entries
  }

  clear() {
    this.entries = []
  }
}
