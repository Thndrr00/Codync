/**
 * The host writes agents' status in plain words (result first, then the real stage, and any failure
 * or delay without waiting for a question). These two helpers shape what the desktop shows: one
 * readable status line, and an approval request's scope, risks and recommendation in one card.
 * Logs, ids and version numbers stay out of the line; nothing from the agent is rewritten into a
 * claim the host didn't make.
 */

const LOG_LINE = /^\s*(?:\[[^\]]*\]|[>$]\s)/
const IDENTIFIER = /\b(?:id|uuid|trace|sha)\s*[:=]\s*\S+/i
const HEX_ID = /^[0-9a-f]{8,}$/i
const VERSION = /\bv?\d+\.\d+(?:\.\d+)?\b/gi

/** The one-line status beside the avatar and in the chat title: a plain sentence, no log noise. */
export function briefStatus(text: string | null | undefined): string {
  const plain = (text ?? '')
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line && !LOG_LINE.test(line) && !IDENTIFIER.test(line) && !HEX_ID.test(line))
  const result = plain[0] ?? ''
  return result.replace(VERSION, '').replace(/\s{2,}/g, ' ').trim()
}

/** The three labelled parts of an approval request, and whatever else the detail carried. */
export interface ApprovalBrief {
  scope: string
  risks: string
  recommendation: string
  rest: string
}

const LABELS: Record<string, keyof Omit<ApprovalBrief, 'rest'>> = {
  scope: 'scope',
  risk: 'risks',
  risks: 'risks',
  recommendation: 'recommendation',
  recommend: 'recommendation',
}

/**
 * Reads `Scope:`, `Risks:` and `Recommendation:` lines wherever they sit in an approval's detail.
 * Other lines are kept for the card's Details disclosure. Returns null unless at least two of the
 * three parts are present, so an unrelated detail stays exactly as it was.
 */
export function approvalBrief(detail: string | null | undefined): ApprovalBrief | null {
  const found: Record<string, string[]> = {}
  const rest: string[] = []
  let current: string | null = null
  for (const line of (detail ?? '').split('\n')) {
    const match = /^\s*(scope|risks?|recommend(?:ation)?)\s*:\s*(.*)$/i.exec(line)
    const key = match ? LABELS[match[1]!.toLowerCase()] : undefined
    if (match && key) {
      current = key
      ;(found[key] ??= []).push(match[2]!.trim())
    } else if (current && !line.trim()) {
      current = null
    } else if (current) {
      found[current]!.push(line.trim())
    } else {
      rest.push(line)
    }
  }
  const parts = ['scope', 'risks', 'recommendation'] as const
  if (parts.filter((part) => found[part]?.some(Boolean)).length < 2) return null
  const value = (part: string) => (found[part] ?? []).filter(Boolean).join(' ')
  return { scope: value('scope'), risks: value('risks'), recommendation: value('recommendation'), rest: rest.join('\n').trim() }
}
