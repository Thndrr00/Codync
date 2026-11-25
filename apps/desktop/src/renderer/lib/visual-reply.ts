/**
 * Client-side decisions for a visual reply: an HTML artifact the agent published above its words
 * (OpenBot `html_render`, agents.md). There is no new service and no new tool surface here; the page
 * is the artifact/file the chat already carries, shown in the existing sandboxed preview.
 * Pure so the bounds and the quote can be tested without a browser.
 */

export interface VisualReply {
  html: string
  truncated: boolean
}

/** The most HTML one inline reply may take, in characters. */
export const VISUAL_REPLY_LIMIT = 2_000_000
/** The most of a selection one quote chip may carry. */
export const QUOTE_LIMIT = 300

/**
 * Decides whether an artifact is a visual reply and bounds it. Plain text and empty files are not
 * pages. A cut never ends inside a tag, so the sanitizer still sees whole markup.
 */
export function visual_reply(html: string | null | undefined, limit: number): VisualReply | null {
  if (!html) return null
  const source = html.trim()
  if (!source || !/<[a-z][\s\S]*>/i.test(source)) return null
  const chars = [...source]
  if (chars.length <= limit) return { html: source, truncated: false }
  let cut = chars.slice(0, Math.max(0, limit)).join('')
  const open = cut.lastIndexOf('<')
  if (open >= 0 && cut.indexOf('>', open) === -1) cut = cut.slice(0, open)
  cut = cut.replace(/&[^;\s]*$/, '')
  return { html: cut, truncated: true }
}

/** A selected passage as one quote chip: whitespace collapsed, bounded, never an empty chip. */
export function quote_text(selection: string | null | undefined, limit: number): string {
  const text = (selection ?? '').replace(/\s+/g, ' ').trim()
  if (!text) return ''
  const chars = [...text]
  return chars.length <= limit ? text : `${chars.slice(0, limit).join('')}…`
}
