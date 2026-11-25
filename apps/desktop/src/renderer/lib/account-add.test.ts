import assert from 'node:assert/strict'
import { test } from 'node:test'
import { accountAdd, signInFailureReason } from './account-add.ts'

test('a second Gemini account offers the code flow, other accounts keep Add account', () => {
  assert.deepEqual(accountAdd({ backendId: 'gemini', hasFirstAccount: true, adding: false }), {
    label: 'Add account with code',
    detail: 'Use the code from the new Google account on its sign-in page. Your current account stays signed in.',
  })
  assert.equal(accountAdd({ backendId: 'gemini', hasFirstAccount: false, adding: false }).label, 'Add account')
  assert.equal(accountAdd({ backendId: 'claude', hasFirstAccount: true, adding: false }).label, 'Add account')
})

test('a sign-in failure says what happened instead of leaking a host error', () => {
  assert.equal(signInFailureReason('Sign-in timed out.'), 'Sign-in timed out. Start again before the code expires.')
  assert.equal(signInFailureReason('cancelled by user'), 'Sign-in was cancelled. Your current account is unchanged.')
  assert.equal(signInFailureReason('that account is already signed in'), 'That account is already signed in. Use a different Google account.')
  assert.equal(signInFailureReason('disk full'), 'disk full')
  assert.equal(signInFailureReason('  '), 'Sign-in did not finish.')
})
