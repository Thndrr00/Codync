import { test } from 'node:test'
import assert from 'node:assert/strict'
import { QUOTE_LIMIT, quote_text, visual_reply } from './visual-reply.ts'

test('an HTML artifact becomes a bounded visual reply', () => {
  assert.equal(visual_reply(null, 100), null)
  assert.equal(visual_reply('', 100), null)
  assert.equal(visual_reply('just words', 100), null, 'plain text is not a page')
  assert.deepEqual(visual_reply('<h1>Hi</h1>', 100), { html: '<h1>Hi</h1>', truncated: false })
  assert.deepEqual(visual_reply('  <p>Hi</p>  ', 100), { html: '<p>Hi</p>', truncated: false })

  const long = visual_reply(`<p>${'a'.repeat(50)}</p><p>b</p>`, 20)
  assert.ok(long)
  assert.equal(long.truncated, true)
  assert.ok(long.html.length <= 20, long.html)
  assert.equal(long.html.includes('<p>b'), false)
})

test('a cut visual reply never ends inside a tag', () => {
  const cut = visual_reply('<div><span>hello</span></div>', 8)
  assert.ok(cut)
  assert.equal(cut.truncated, true)
  assert.equal(cut.html, '<div>', 'an unfinished tag is dropped: ' + cut.html)
})

test('a selection becomes one bounded quote chip', () => {
  assert.equal(quote_text('', QUOTE_LIMIT), '')
  assert.equal(quote_text('   \n ', QUOTE_LIMIT), '')
  assert.equal(quote_text('  a\n  b  ', QUOTE_LIMIT), 'a b')
  assert.equal(quote_text('x'.repeat(QUOTE_LIMIT), QUOTE_LIMIT), 'x'.repeat(QUOTE_LIMIT))
  const long = quote_text('y'.repeat(QUOTE_LIMIT + 10), QUOTE_LIMIT)
  assert.equal([...long].length, QUOTE_LIMIT + 1)
  assert.ok(long.endsWith('…'))
  assert.equal(quote_text(null, QUOTE_LIMIT), '')
})
