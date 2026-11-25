import assert from 'node:assert/strict'
import { test } from 'node:test'
import { pageNotice } from './page-notice.ts'

test('a published page becomes an Open page notice with its title', () => {
  assert.deepEqual(pageNotice({ kind: 'agent', data: { artifact: { id: 'art-1', title: 'Quarterly report' } } }), {
    label: 'Open page', href: 'art-1', title: 'Quarterly report', thread: null,
  })
})

test('a link in the agent answer is offered, and the notice stays in its side thread', () => {
  const notice = pageNotice({ kind: 'agent', threadId: 'thread-7', data: { text: 'Open https://example.com/x to continue.' } })
  assert.deepEqual(notice, { label: 'Open page', href: 'https://example.com/x', title: 'https://example.com/x', thread: 'thread-7' })
})

test('a user message, a tool row and an empty answer show no notice', () => {
  assert.equal(pageNotice(null), null)
  assert.equal(pageNotice({ kind: 'user', data: { artifact: { id: 'a', title: 'x' } } }), null)
  assert.equal(pageNotice({ kind: 'tool', data: { text: 'https://example.com' } }), null)
  assert.equal(pageNotice({ kind: 'agent', data: { text: 'Done.' } }), null)
})
