// A web link clicked in a chat carries the app that opened it, without changing links that
// already tagged a source, local addresses, or addresses whose query is a sign-in or a signature.

const SENSITIVE_KEYS = new Set([
  'sig',
  'signature',
  'token',
  'access_token',
  'id_token',
  'refresh_token',
  'auth',
  'authorization',
  'code',
  'state',
  'key',
  'nonce',
  'oauth',
  'sso',
  'login',
  'signin',
  'sign-in',
  'verify',
  'reset',
  'expires',
  'x-amz-signature',
  'x-goog-signature',
])

function isLocal(host: string): boolean {
  const h = host.toLowerCase().replace(/^\[|\]$/g, '')
  return h === 'localhost' || h === '::1' || h === '0.0.0.0' || h.endsWith('.localhost') || h.endsWith('.local') || /^127\./.test(h)
}

/** Adds `utm_source=codync` to a web link, leaving other schemes and sensitive links untouched. */
export function withChatLinkSource(url: string): string {
  let parsed: URL
  try {
    parsed = new URL(url)
  } catch {
    return url
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return url
  if (isLocal(parsed.hostname)) return url
  for (const [name] of parsed.searchParams) {
    const lower = name.toLowerCase()
    if (lower === 'utm_source' || SENSITIVE_KEYS.has(lower)) return url
  }
  const hashAt = url.indexOf('#')
  const before = hashAt < 0 ? url : url.slice(0, hashAt)
  const hash = hashAt < 0 ? '' : url.slice(hashAt)
  const joiner = before.includes('?') ? (before.endsWith('?') || before.endsWith('&') ? '' : '&') : '?'
  return `${before}${joiner}utm_source=codync${hash}`
}
