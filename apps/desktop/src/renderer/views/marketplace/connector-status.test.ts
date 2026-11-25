import assert from 'node:assert/strict'
import test from 'node:test'
import { connectorStatus, SIGN_IN_NEEDED, signInFieldsOnFailure, verifyOutcome } from './connector-status'
import type { InstalledConnector } from './market-models'

function connector(patch: Partial<InstalledConnector> = {}): InstalledConnector {
  return { id: 'm', name: 'Mail', description: '', kind: 'remote', keys: [], ...patch }
}

test('a saved sign-in that still needs a person is marked Sign-in needed', () => {
  assert.deepEqual(connectorStatus(connector({ auth: 'signedOut' })), { label: SIGN_IN_NEEDED, needsSignIn: true })
  assert.deepEqual(
    connectorStatus(connector({ auth: 'signedIn', lastFailure: { kind: 'auth', message: 'expired', at: 1 } })),
    { label: SIGN_IN_NEEDED, needsSignIn: true }
  )
  assert.deepEqual(connectorStatus(connector({ url: 'https://example.com/mcp' })), {
    label: 'https://example.com/mcp',
    needsSignIn: false,
  })
})

test('sign-in fields appear only under an auth failure on a remote server', () => {
  assert.deepEqual(signInFieldsOnFailure(connector({ url: 'https://x.test/mcp', keys: ['token', 'client_id'] })), [] , 'no failure means no fields')
  const failed = connector({
    url: 'https://x.test/mcp',
    keys: ['token', 'client_id'],
    lastFailure: { kind: 'auth', message: 'expired', at: 1 },
  })
  assert.deepEqual(signInFieldsOnFailure(failed), ['client_id', 'token'])
  assert.deepEqual(signInFieldsOnFailure({ ...failed, url: null }), [], 'a local server has no sign-in fields')
  assert.deepEqual(signInFieldsOnFailure({ ...failed, lastFailure: { kind: 'unreachable', message: 'offline', at: 1 } }), [])
})

test('a check on a signed-out server asks for sign-in instead of claiming failure', () => {
  assert.match(verifyOutcome(connector({ auth: 'signedOut' }), false), /Sign in/)
  assert.equal(verifyOutcome(connector(), true), 'Connected')
  assert.match(verifyOutcome(connector(), false), /failed/)
})
