/**
 * Pure wording for a connector's row and its Test connection result (OpenBot 15267/15864).
 * A saved sign-in is not a working one: an `auth` failure or a stored-but-signed-out server is
 * marked "Sign-in needed" instead of showing a generic failure. The sign-in fields offered after
 * a failed check are the connector's own keys — no new setting is introduced here.
 */

import type { InstalledConnector } from './market-models'

export const SIGN_IN_NEEDED = 'Sign-in needed'
export const SIGN_IN_FIELDS_NOTE = 'If this server needs a sign-in app, add its client id and secret here.'
export const TEST_CONNECTION_SIGN_IN = 'Sign in to this connection before checking it.'

export interface ConnectorStatus {
  label: string
  needsSignIn: boolean
}

/** What the installed row says, and whether it needs a person to sign in. */
export function connectorStatus(c: InstalledConnector): ConnectorStatus {
  if (c.lastFailure?.kind === 'auth' || c.auth === 'signedOut') return { label: SIGN_IN_NEEDED, needsSignIn: true }
  if (c.lastFailure) return { label: `${c.lastFailure.kind.toUpperCase()}: ${c.lastFailure.message}`, needsSignIn: false }
  return { label: c.command ?? c.url ?? c.description ?? '', needsSignIn: false }
}

/**
 * The fields shown under a failed Test connection. Only an auth failure on a remote server that
 * actually saved fields gets them; a command server's env keys and every other failure keep their
 * own row. Values are never in this list, only their names.
 */
export function signInFieldsOnFailure(c: InstalledConnector): string[] {
  if (c.lastFailure?.kind !== 'auth' || !c.url) return []
  return [...(c.keys ?? [])].sort()
}

/** The text shown when a check on a signed-out server is skipped, not attempted. */
export function verifyOutcome(c: InstalledConnector, ok: boolean): string {
  if (ok) return 'Connected'
  return c.auth === 'signedOut' ? TEST_CONNECTION_SIGN_IN : 'Test connection failed. Check the server and its sign-in.'
}
