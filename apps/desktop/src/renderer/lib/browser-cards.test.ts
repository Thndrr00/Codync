import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  MAX_SIGN_IN_URL,
  PAGE_NOT_ANSWERING,
  SIGN_IN_FROM_DESKTOP,
  SIGN_IN_ON_THIS_COMPUTER,
  SIGN_IN_PAGE_UNAVAILABLE,
  SIGN_IN_WAITING,
  STALL_MS,
  browserNotice,
  cardLife,
  cardStaysOpen,
  finishSignInAddress,
  pageStalled,
  signInCardAction,
  START_NEW_SIGN_IN,
} from './browser-cards.ts'

test('a takeover card stays answerable after the agent stops waiting', () => {
  assert.equal(cardLife(true, false, false), 'keep-open')
  assert.equal(cardLife(false, false, false), 'answer-as-next-message')
  assert.equal(cardStaysOpen(false, false, false), true, 'the answer becomes the next message')
  assert.equal(cardLife(false, false, true), 'close')
  assert.equal(cardLife(false, true, false), 'close', 'the card deadline still ends it')
})

test('a host notice appears only when the host sent words', () => {
  assert.equal(browserNotice(START_NEW_SIGN_IN), START_NEW_SIGN_IN)
  assert.equal(browserNotice('  '), null)
  assert.equal(browserNotice(null), null)
  assert.equal(browserNotice(undefined), null)
  assert.equal(browserNotice(PAGE_NOT_ANSWERING), PAGE_NOT_ANSWERING)
  assert.equal(browserNotice(SIGN_IN_PAGE_UNAVAILABLE), SIGN_IN_PAGE_UNAVAILABLE)
})

test('a refused sign-in card has one action and never opens the server page', () => {
  assert.equal(signInCardAction(false, false), null, 'a normal card keeps its actions')
  assert.deepEqual(signInCardAction(true, false), {
    action: 'signInOnThisComputer',
    label: SIGN_IN_ON_THIS_COMPUTER,
    showOpen: false,
  })
  assert.deepEqual(signInCardAction(true, true), {
    action: 'signInFromDesktop',
    label: SIGN_IN_FROM_DESKTOP,
    showOpen: false,
  })
})

test('a page is stalled only after a whole silent window', () => {
  const now = 100_000
  assert.equal(pageStalled(null, now), false, 'no frame yet is not a stall')
  assert.equal(pageStalled(now - STALL_MS + 1, now), false)
  assert.equal(pageStalled(now - STALL_MS, now), true)
  assert.equal(pageStalled(now - STALL_MS * 2, now), true)
  assert.equal(pageStalled(now + 10, now), false, 'a backwards clock is not a stall')
})

test('a pasted address finishes the sign-in with only state, code and error', () => {
  assert.deepEqual(finishSignInAddress('https://other.example/callback?state=abc&code=xyz'), {
    state: 'abc',
    code: 'xyz',
    error: null,
  })
  assert.deepEqual(finishSignInAddress('codync://oauth#state=abc&code=xyz'), {
    state: 'abc',
    code: 'xyz',
    error: null,
  })
  assert.deepEqual(finishSignInAddress(' https://other.example/callback?state=abc&error=denied '), {
    state: 'abc',
    code: null,
    error: 'denied',
  })
})

test('a bad pasted address is refused before it leaves the computer', () => {
  assert.equal(finishSignInAddress(''), null)
  assert.equal(finishSignInAddress('not an address'), null)
  assert.equal(finishSignInAddress('javascript:alert(1)'), null)
  assert.equal(finishSignInAddress('https://other.example/callback?code=xyz'), null, 'no state')
  assert.equal(finishSignInAddress('https://other.example/callback?state=abc'), null, 'no result')
  assert.equal(finishSignInAddress(`https://x.example/${'a'.repeat(MAX_SIGN_IN_URL)}`), null)
})

test('the waiting label is a fixed host string', () => {
  assert.equal(SIGN_IN_WAITING, 'Waiting for the sign-in to finish in the browser.')
})
