import { test } from 'node:test'
import assert from 'node:assert/strict'
import { documentSlash, DOCUMENT_BLOCKS } from './document-blocks.ts'

test('the slash menu includes the four catalog commands and filters prefixes', () => {
  assert.deepEqual(documentSlash('/r', 2)?.items.map((item) => item.command), ['routine'])
  for (const command of ['doc', 'routine', 'skill', 'task']) {
    assert.ok(documentSlash('/', 1)?.items.some((item) => item.command === command))
  }
})

test('slash text in prose, indented code, selections or unfinished lines stays literal', () => {
  for (const text of ['path/doc', '  /doc', '/unknown', '/Doc', '/doc extra']) assert.equal(documentSlash(text, text.length), null)
  assert.equal(documentSlash('/routine', 2), null)
  for (const cursor of [-1, 1.5, 50]) assert.equal(documentSlash('/doc', cursor), null)
})

test('fenced examples stay literal, including longer fences and mixed markers', () => {
  for (const prefix of ['```md\n', '~~~~\n', '````\n```\n', '```\n~~~\n', '```\n```text\n']) {
    const text = prefix + '/doc'
    assert.equal(documentSlash(text, text.length), null)
  }
  const closed = '```md\r\n/doc\r\n```\r\n/doc'
  assert.equal(documentSlash(closed, closed.length)?.items[0].command, 'doc')
})

test('inserting a block preserves surrounding Unicode and Markdown bytes', () => {
  const text = '日本語 🐈\r\n/task\r\nTail'
  const cursor = text.indexOf('\r\nTail')
  const menu = documentSlash(text, cursor)!
  assert.equal(text.slice(0, menu.start) + menu.items[0].markdown + text.slice(menu.end), '日本語 🐈\r\n- [ ] Task\n\r\nTail')
  assert.equal(new Set(DOCUMENT_BLOCKS.map((block) => block.command)).size, DOCUMENT_BLOCKS.length)
})
