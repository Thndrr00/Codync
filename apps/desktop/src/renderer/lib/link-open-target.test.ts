import assert from 'node:assert/strict'
import { test } from 'node:test'
import { linkOpenTarget } from './link-open-target.ts'

test('web links go to the default browser, Codync links stay in the app', () => {
  assert.deepEqual(linkOpenTarget('https://example.com/a?b=1'), { target: 'browser', detail: 'Opens in your default browser.' })
  assert.deepEqual(linkOpenTarget('HTTP://example.com'), { target: 'browser', detail: 'Opens in your default browser.' })
  assert.deepEqual(linkOpenTarget('codync://marketplace/apps/x'), { target: 'app', detail: 'Opens inside Codync.' })
})

test('local HTML and PDF get the in-app preview, other files the system app', () => {
  assert.equal(linkOpenTarget('/Users/me/report.pdf').target, 'preview')
  assert.equal(linkOpenTarget('file:///C:/site/index.html').target, 'preview')
  assert.equal(linkOpenTarget('C:\\docs\\notes.docx').target, 'system')
  assert.equal(linkOpenTarget('mailto:me@example.com').target, 'system')
})

test('a blank or unknown link never claims it opens in-app', () => {
  assert.equal(linkOpenTarget('').target, 'system')
  assert.equal(linkOpenTarget('  ').target, 'system')
  assert.equal(linkOpenTarget('javascript:alert(1)').target, 'system')
})
