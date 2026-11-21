import { test } from 'node:test'
import assert from 'node:assert/strict'
import { safePageHtml } from './page-preview.ts'

test('a preview drops scripts, handlers and frames', () => {
  const html = '<script>alert(1)</script><div onclick="alert(2)"><iframe src="https://x"></iframe>Hi</div>'
  const safe = safePageHtml(html)
  assert.equal(safe.includes('script'), false)
  assert.equal(safe.includes('onclick'), false)
  assert.equal(safe.includes('iframe'), false)
  assert.equal(safe.includes('Hi'), true)
})

test('a preview cannot fetch from the network', () => {
  const safe = safePageHtml('<link href="https://cdn/x.css"><img src="//cdn/x.png"><style>@import url(https://x); .a{background:url("https://x/y")}</style><a href="#top">Top</a>')
  assert.equal(safe.includes('https://'), false)
  assert.equal(safe.includes('//cdn'), false)
  assert.equal(safe.includes('@import'), false)
  assert.equal(safe.includes('url("https://x/y")'), false)
  assert.equal(safe.includes('href="#top"'), true)
})
