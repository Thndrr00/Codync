/**
 * A model that refuses a chat again for too many images gets one bounded, actionable hint
 * instead of another "Try again". The hint is a fixed string, so an agent's long error text
 * never grows the card.
 */
const HINT = 'Too many images. Start a new chat and ask the agent to open fewer images per step, or choose a model that reads files.'

const IMAGE_LIMIT = /\btoo many images?\b|\bimage (?:count )?limit\b|\bmax(?:imum)? images?\b/i

export function image_limit_hint(error: string): string | null {
  const text = (error ?? '').trim()
  if (!text || !IMAGE_LIMIT.test(text)) return null
  return HINT
}
