import test from 'node:test'
import assert from 'node:assert/strict'
import { withChatLinkSource } from './link-source.ts'

test('a plain web link gets the source appended', () => {
  assert.equal(withChatLinkSource('https://example.com/docs'), 'https://example.com/docs?utm_source=codync')
})

test('an existing query, a trailing separator and a fragment keep their shape', () => {
  assert.equal(withChatLinkSource('https://example.com/p?a=1'), 'https://example.com/p?a=1&utm_source=codync')
  assert.equal(withChatLinkSource('https://example.com/p?'), 'https://example.com/p?utm_source=codync')
  assert.equal(withChatLinkSource('https://example.com/p?a=1&'), 'https://example.com/p?a=1&utm_source=codync')
  assert.equal(withChatLinkSource('https://example.com/p#part'), 'https://example.com/p?utm_source=codync#part')
  assert.equal(withChatLinkSource('https://example.com/p?a=1#part'), 'https://example.com/p?a=1&utm_source=codync#part')
})

test('a link that already carries a source is left alone', () => {
  for (const url of [
    'https://example.com/p?utm_source=other',
    'https://example.com/p?a=1&UTM_SOURCE=other',
    'https://example.com/p?utm_source=',
  ]) {
    assert.equal(withChatLinkSource(url), url)
  }
})

test('local addresses are left alone', () => {
  for (const url of [
    'http://localhost:3000/p',
    'http://127.0.0.1:8080/p',
    'http://[::1]:3000/p',
    'https://my-box.local/p',
    'https://printer.localhost/p',
  ]) {
    assert.equal(withChatLinkSource(url), url)
  }
})

test('sign-in and signed links are left alone', () => {
  for (const url of [
    'https://example.com/auth?code=abc&state=xyz',
    'https://example.com/login?token=abc',
    'https://example.com/verify?signature=abc',
    'https://example.com/reset?key=abc',
    'https://files.example.com/f?X-Amz-Signature=abc',
    'https://example.com/p?access_token=abc#frag',
  ]) {
    assert.equal(withChatLinkSource(url), url)
  }
})

test('non-web links and malformed links are left alone', () => {
  for (const url of ['mailto:ada@example.com', 'tel:+15550000', 'file:///tmp/a', 'codync://bot/1', 'not a url']) {
    assert.equal(withChatLinkSource(url), url)
  }
})
