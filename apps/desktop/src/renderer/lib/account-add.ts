/**
 * How to add a second account for an agent (OpenBot 0.73.0's "Add account with code"). Gemini's
 * Google sign-in can keep the first account signed in, so a second one is added with a code from its
 * page. The failure text is classified into a reason the person can act on, not a raw host string.
 */
export interface AccountAddInput {
  backendId: string
  /** The computer's own sign-in, or the count of isolated accounts already here. */
  hasFirstAccount: boolean
  adding: boolean
}

export interface AccountAddDecision {
  label: string
  detail: string
}

export function accountAdd(input: AccountAddInput): AccountAddDecision {
  const second = input.hasFirstAccount && input.backendId === 'gemini'
  return second
    ? { label: 'Add account with code', detail: 'Use the code from the new Google account on its sign-in page. Your current account stays signed in.' }
    : { label: 'Add account', detail: 'Sign in to another account without signing out of this one.' }
}

/** A cancelled, timed-out or refused sign-in gets a reason; anything else keeps the host's own text. */
export function signInFailureReason(message: string): string {
  const value = message.trim()
  if (/time[d\s-]*out|timed out/i.test(value)) return 'Sign-in timed out. Start again before the code expires.'
  if (/cancel|abort/i.test(value)) return 'Sign-in was cancelled. Your current account is unchanged.'
  if (/already signed in|same account/i.test(value)) return 'That account is already signed in. Use a different Google account.'
  if (/another sign-in/i.test(value)) return 'Another sign-in was started. Finish it before adding this account.'
  return value || 'Sign-in did not finish.'
}
