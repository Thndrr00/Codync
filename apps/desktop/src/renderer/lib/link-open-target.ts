import { previewKind } from './chat-file-link.ts'

/**
 * Where a link in a chat message goes (OpenBot 0.73.0's in-app vs default-browser clarification).
 * Pure: the renderer shows the same explanation in Settings and follows it when the link is clicked.
 */
export type LinkOpenTarget = 'app' | 'preview' | 'browser' | 'system'

export interface LinkOpenDecision {
  target: LinkOpenTarget
  detail: string
}

export function linkOpenTarget(href: string): LinkOpenDecision {
  const value = href.trim()
  if (/^(codync|codync-dev):/i.test(value)) return { target: 'app', detail: 'Opens inside Codync.' }
  if (/^https?:\/\//i.test(value)) return { target: 'browser', detail: 'Opens in your default browser.' }
  if (/^mailto:/i.test(value)) return { target: 'system', detail: 'Opens in your mail app.' }
  const path = /^file:\/\//i.test(value) ? decodeURIComponent(value.slice(7)) : value
  if (previewKind(path)) return { target: 'preview', detail: 'Opens in Codync’s built-in page preview.' }
  return { target: 'system', detail: 'Opens in the app your computer uses for this file.' }
}
