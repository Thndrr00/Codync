import assert from 'node:assert/strict'
import { test } from 'node:test'
import { browserCardSubtitle } from './island-browser-card.ts'

test('Open on a page card says it opens the page in the main window', () => {
  assert.equal(
    browserCardSubtitle('Agent needs a step on a web page', 'Complete the CAPTCHA', [{ label: 'Open' }, { label: 'Done' }]),
    'Open shows the page in the main window, where you can type.',
  )
})

test('a page card without Open still says the answer reaches the agent', () => {
  assert.equal(
    browserCardSubtitle('Sign in to the site', 'Sign in on this computer', []),
    'Your answer still goes to the agent, even if it stopped waiting.',
  )
})

test('an unrelated card gets no browser hint', () => {
  assert.equal(browserCardSubtitle('Approve a file change', 'Edit main.rs', [{ label: 'Allow' }]), null)
})
