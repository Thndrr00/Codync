/**
 * A header name is an HTTP token: letters, digits and a small set of separators,
 * never a space, colon, slash or bracket. Ported from OpenBot nightly
 * triage-13174 so the form, the host and the agent refuse the same names.
 */
const TOKEN = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/

export const HEADER_NAME_MESSAGE =
  'Header names cannot contain spaces or symbols such as : ( ) /. Use a name such as Authorization, and put the key or token in Value.'

export function validHeaderName(name: string): boolean {
  return TOKEN.test(name) && name.length <= 64
}

/** The first unusable key in a "Name: value" text box, or null when every row is fine. Blank rows are skipped. */
export function invalidHeaderKey(text: string): string | null {
  for (const line of text.split('\n')) {
    const at = line.indexOf(':')
    if (at < 0) continue
    const key = line.slice(0, at).trim()
    if (key && !validHeaderName(key)) return key
  }
  return null
}
