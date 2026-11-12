import assert from 'node:assert/strict'
import { test } from 'node:test'
import { existingBrowser, focusExistingBrowser } from '../../../main/existing-browser.ts'

test('missing and destroyed browser windows report unavailable without any focus or creation actions', () => {
  assert.equal(existingBrowser(null), false)
  assert.equal(focusExistingBrowser(null), false)
  const calls: string[] = []
  const destroyed = { isDestroyed: () => true, show: () => calls.push('show'), focus: () => calls.push('focus'), restore: () => calls.push('restore'), open: () => assert.fail('must not create a browser'), loadURL: () => assert.fail('must not navigate') }
  assert.equal(existingBrowser(destroyed), false)
  assert.equal(focusExistingBrowser(destroyed), false)
  assert.deepEqual(calls, [])
})

test('existing minimized windows restore before show and focus without creating or navigating anything', () => {
  const calls: string[] = []
  const window = {
    isDestroyed: () => false, isMinimized: () => true,
    restore: () => calls.push('restore'), show: () => calls.push('show'), focus: () => calls.push('focus'),
    open: () => assert.fail('must not open a new window'), createTab: () => assert.fail('must not create a tab'),
    loadURL: () => assert.fail('must not navigate'), browserPanelOpen: () => assert.fail('must not acquire a lease'),
  }
  assert.equal(existingBrowser(window), true)
  assert.deepEqual(calls, [])
  assert.equal(focusExistingBrowser(window), true)
  assert.deepEqual(calls, ['restore', 'show', 'focus'])
})

test('an ordinary retained window needs only show and focus', () => {
  const calls: string[] = []
  const window = { isDestroyed: () => false, show: () => calls.push('show'), focus: () => calls.push('focus') }
  assert.equal(focusExistingBrowser(window), true)
  assert.deepEqual(calls, ['show', 'focus'])
  calls.length = 0
  assert.equal(focusExistingBrowser({ ...window, isMinimized: () => false, restore: () => assert.fail('not minimized') }), true)
  assert.deepEqual(calls, ['show', 'focus'])
})

test('a window inspection or restore failure cannot be presented as successful focus', () => {
  const fail = () => { throw new Error('Window operation failed') }
  const window = { isDestroyed: () => false, show: () => assert.fail('show must not follow failed restore'), focus: () => assert.fail('focus must not follow failed restore'), isMinimized: () => true, restore: fail }
  assert.throws(() => existingBrowser({ ...window, isDestroyed: fail }), /Window operation failed/)
  assert.throws(() => focusExistingBrowser(window), /Window operation failed/)
})

test('show and focus exceptions propagate instead of claiming the browser was revealed', () => {
  const fail = () => { throw new Error('Window disappeared') }
  assert.throws(() => focusExistingBrowser({ isDestroyed: () => false, show: fail, focus: () => assert.fail('focus must not follow failed show') }), /Window disappeared/)
  assert.throws(() => focusExistingBrowser({ isDestroyed: () => false, show() {}, focus: fail }), /Window disappeared/)
})
