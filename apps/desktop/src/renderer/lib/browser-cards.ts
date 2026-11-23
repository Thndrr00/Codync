/**
 * Client-side decisions for the browser's sign-in and takeover cards (OpenBot 0.77.0).
 *
 * The host owns the card words and sends them as fixed labels; nothing here reads page content or
 * sends an address to analytics. These helpers only decide *when* a notice is shown, whether a page
 * has stopped answering, and which action a refused sign-in card offers.
 */

export const START_NEW_SIGN_IN =
  'This sign-in is no longer active. Go back to Codync and start a new sign-in.'
export const SIGN_IN_WAITING = 'Waiting for the sign-in to finish in the browser.'
export const PAGE_NOT_ANSWERING =
  'This page is not answering; a script may still be running. Reload it to continue.'
export const SIGN_IN_PAGE_UNAVAILABLE =
  'The sign-in page could not load. Check the internet connection and open the sign-in window again.'
export const SIGN_IN_ON_THIS_COMPUTER = 'Sign in on this computer'
export const SIGN_IN_FROM_DESKTOP = 'Sign in from the desktop app'

/** A page quiet for this long may be stalled, below the 30 s a chat tool call has. */
export const STALL_MS = 20_000
/** Bounds a pasted sign-in address. */
export const MAX_SIGN_IN_URL = 2048

export type CardLife = 'close' | 'keep-open' | 'answer-as-next-message'

/**
 * 0.77.0: a takeover/ask-help card stays answerable after the agent stops waiting; the answer goes
 * to the agent as the next message. Only the card's own deadline or an answer closes it.
 */
export function cardLife(waiting: boolean, expired: boolean, answered: boolean): CardLife {
  if (expired || answered) return 'close'
  return waiting ? 'keep-open' : 'answer-as-next-message'
}

export function cardStaysOpen(waiting: boolean, expired: boolean, answered: boolean): boolean {
  return cardLife(waiting, expired, answered) !== 'close'
}

/** A host notice is shown only when it has words; an empty label means no notice. */
export function browserNotice(text: string | null | undefined): string | null {
  const trimmed = text?.trim()
  return trimmed ? trimmed : null
}

export interface SignInCardAction {
  action: 'signInOnThisComputer' | 'signInFromDesktop'
  label: string
  showOpen: false
}

/**
 * 0.77.0: when a site refuses a server sign-in the card keeps one action and never opens the
 * server's browser preview. In a web browser it drops Open and points to the desktop app.
 */
export function signInCardAction(serverRefused: boolean, web: boolean): SignInCardAction | null {
  if (!serverRefused) return null
  return web
    ? { action: 'signInFromDesktop', label: SIGN_IN_FROM_DESKTOP, showOpen: false }
    : { action: 'signInOnThisComputer', label: SIGN_IN_ON_THIS_COMPUTER, showOpen: false }
}

/** Whether a tab that has sent no frame for a while should offer a Reload action. */
export function pageStalled(lastFrameAt: number | null, now: number, windowMs = STALL_MS): boolean {
  if (lastFrameAt === null) return false
  return Math.max(0, now - lastFrameAt) >= windowMs
}

export interface SignInAddress {
  state: string
  code: string | null
  error: string | null
}

/**
 * 0.77.0: finish an app sign-in started on a phone or another computer by pasting the address its
 * browser shows at the end. Parses only `state`, `code` and `error` for the existing connector OAuth
 * finish path; rejects anything else before it leaves this computer.
 */
export function finishSignInAddress(raw: string): SignInAddress | null {
  const value = raw.trim()
  if (!value || value.length > MAX_SIGN_IN_URL) return null
  let url: URL
  try {
    url = new URL(value)
  } catch {
    return null
  }
  if (!['https:', 'http:', 'codync:'].includes(url.protocol)) return null
  const fields = [...url.searchParams, ...(url.hash.includes('=') ? new URLSearchParams(url.hash.slice(1)) : [])]
  const get = (key: string) => fields.find(([name]) => name === key)?.[1] ?? null
  const state = get('state')
  const code = get('code')
  const error = get('error') ?? get('error_description')
  if (!state || (!code && !error)) return null
  return { state, code, error }
}
