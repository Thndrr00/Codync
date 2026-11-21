import assert from 'node:assert/strict'
import { test } from 'node:test'
import { safeSignInUrl } from './sign-in-url.ts'

const signIn = 'https://auth.example.com/signin?client_id=app&redirect_uri=https%3A%2F%2Fcodync.dev%2Fdone'

test('a plain https sign-in link is safe to copy', () => {
  assert.equal(safeSignInUrl(signIn), signIn)
  assert.equal(safeSignInUrl('https://example.com/login'), 'https://example.com/login')
})

test('a link that carries a token is never copied', () => {
  for (const name of ['token', 'access_token', 'code', 'secret', 'api_key', 'client_secret', 'session', 'SIG']) {
    assert.equal(safeSignInUrl(`https://auth.example.com/signin?${name}=one-time-secret`), null, `${name} hides a secret`)
  }
  assert.equal(safeSignInUrl(`${signIn}#token=one-time-secret`), null, 'a fragment can hide a secret too')
  assert.equal(safeSignInUrl('https://auth.example.com/callback?code=abc&state=xyz'), null, 'an OAuth code is a token')
})

test('an untrustworthy address is never copied', () => {
  assert.equal(safeSignInUrl('http://auth.example.com/signin'), null, 'only https')
  assert.equal(safeSignInUrl('javascript:alert(1)'), null, 'not an address')
  assert.equal(safeSignInUrl('/signin'), null, 'not absolute')
  assert.equal(safeSignInUrl('https://user:pass@auth.example.com/signin'), null, 'credentials in the URL')
  assert.equal(safeSignInUrl('https://auth.example.com/sign in'), null, 'control characters')
  assert.equal(safeSignInUrl('https://auth.example.com/' + 'a'.repeat(9000)), null, 'bounded length')
  assert.equal(safeSignInUrl(null), null)
  assert.equal(safeSignInUrl(42), null)
})
