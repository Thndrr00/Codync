/**
 * What to offer for text selected in a chat message (OpenBot 0.70.0's Quote / Explain / Improve /
 * Describe). A selection in a table cell or one holding a link is still plain text to the DOM, so the
 * same four actions cover both; they go back to the side thread the text came from, never to main chat.
 */
export type SelectionActionKind = 'quote' | 'explain' | 'improve' | 'describe'

export interface SelectionAction {
  kind: SelectionActionKind
  label: string
  icon: string
  prompt: string
  /** The thread the message belongs to; `null` is the bot's main chat. */
  thread: string | null
}

const MAX_SELECTION = 2000

/** A selection that wraps a web link is still actionable; the link is kept in the quote. */
export function selectionHasLink(text: string): boolean {
  return /https?:\/\/\S+/i.test(text)
}

/**
 * Pure decision: four actions for a non-empty selection, none for whitespace. Long selections are
 * cut so a whole document cannot be pasted into a prompt by accident.
 */
export function selectionActions(text: string, thread: string | null): SelectionAction[] {
  const selected = text.trim()
  if (!selected) return []
  const quoted = selected.length > MAX_SELECTION ? `${selected.slice(0, MAX_SELECTION)}…` : selected
  const prefix = (verb: string) => `${verb}:\n\n> ${quoted}`
  return [
    { kind: 'quote', label: 'Quote', icon: 'text.quote', prompt: `> ${quoted}`, thread },
    { kind: 'explain', label: 'Explain', icon: 'questionmark.circle', prompt: prefix('Explain this'), thread },
    { kind: 'improve', label: 'Improve', icon: 'wand.and.stars', prompt: prefix('Improve this text'), thread },
    { kind: 'describe', label: 'Describe', icon: 'photo', prompt: prefix('Describe this'), thread },
  ]
}
