/**
 * Ported from OpenBot nightly: a provider's sign-in page can be opened on the computer, and the
 * same link offered as Copy so it can be opened in a browser here. Only a plain https link is
 * copied: a URL that already carries a one-time code, token or credentials is never put on the
 * clipboard. This is pure device-side validation — the host still owns the sign-in flow.
 */

const MAX_URL = 8_192
/** Query or fragment names that carry a secret. Compared without case, with `_` optional. */
const SECRET_KEY = /^(access_?token|refresh_?token|token|code|secret|client_?secret|api_?key|apikey|key|sig|signature|password|passwd|pwd|authorization|session)$/i
const CONTROL = /[\u0000-\u001f\u007f\s]/

function secretName(value: string): boolean {
  return SECRET_KEY.test(value.split('=')[0] ?? '')
}

/** The URL safe to copy, or null when it could hide a token. */
export function safeSignInUrl(value: unknown): string | null {
  if (typeof value !== 'string' || !value || value.length > MAX_URL || CONTROL.test(value)) return null
  let url: URL
  try {
    url = new URL(value)
  } catch {
    return null
  }
  if (url.protocol !== 'https:' || !url.hostname || url.username || url.password) return null
  for (const [name] of url.searchParams) if (secretName(name)) return null
  const fragment = url.hash.replace(/^#/, '')
  if (fragment && secretName(fragment)) return null
  return url.toString()
}
