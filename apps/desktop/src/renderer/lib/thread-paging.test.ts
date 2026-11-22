import assert from 'node:assert/strict'
import { test } from 'node:test'
import { mergeThreadPage, THREAD_PAGE_SIZE, threadPageCursor, type ThreadPage } from './thread-paging.ts'

const row = (id: string, updatedAt: number) => ({ id, updatedAt })

test('a full archived page offers another page and keeps the rows already shown', () => {
  const first = Array.from({ length: THREAD_PAGE_SIZE }, (_, i) => row(`t${i}`, 100 - i))
  const second = [row('older', 1)]
  const page: ThreadPage<ReturnType<typeof row>> = mergeThreadPage(first, second)
  assert.equal(page.hasMore, false)
  assert.equal(page.threads.length, THREAD_PAGE_SIZE + 1)
  assert.deepEqual(page.threads.map((t) => t.id), [...first.map((t) => t.id), 'older'])
  assert.equal(threadPageCursor(page.threads), 'older')
})

test('an older host that ignores the cursor stops the paging button', () => {
  const first = Array.from({ length: THREAD_PAGE_SIZE }, (_, i) => row(`t${i}`, 100 - i))
  assert.equal(mergeThreadPage(first, first).hasMore, false)
  assert.equal(mergeThreadPage([], first).hasMore, true)
})

test('a partial page has no older threads and the rows stay newest first', () => {
  const page = mergeThreadPage([row('new', 200)], [row('old', 100), row('older', 50)], THREAD_PAGE_SIZE)
  assert.equal(page.hasMore, false)
  assert.deepEqual(page.threads.map((t) => t.id), ['new', 'old', 'older'])
})

test('an empty list has no cursor', () => {
  assert.equal(threadPageCursor([]), null)
})
