import { test } from 'node:test'
import assert from 'node:assert/strict'
import { JSDOM } from 'jsdom'
import { Editor } from '@tiptap/core'
import { documentExtensions, documentLink, richDocumentSlash } from './document-editor.ts'

const dom = new JSDOM('<!doctype html><html><body></body></html>', { url: 'https://localhost/' })
for (const [key, value] of Object.entries({
  window: dom.window, document: dom.window.document, navigator: dom.window.navigator,
  Node: dom.window.Node, HTMLElement: dom.window.HTMLElement, Element: dom.window.Element,
  MutationObserver: dom.window.MutationObserver, DOMParser: dom.window.DOMParser,
})) {
  Object.defineProperty(globalThis, key, { value, configurable: true })
}
Object.defineProperty(globalThis, 'getComputedStyle', { value: dom.window.getComputedStyle.bind(dom.window), configurable: true })

function editor(content: string) {
  return new Editor({ element: document.createElement('div'), extensions: documentExtensions(), content, contentType: 'markdown' })
}

test('visual formatting serializes to Markdown and survives reopening', () => {
  const first = editor('Hello world')
  try {
    first.commands.setTextSelection({ from: 1, to: 6 })
    first.commands.toggleBold()
    first.commands.setLink({ href: 'https://example.com' })
    const markdown = first.getMarkdown()
    assert.match(markdown, /\*\*Hello\*\*/)
    assert.match(markdown, /https:\/\/example.com/)
    const second = editor(markdown)
    try { assert.deepEqual(second.getJSON(), first.getJSON()) } finally { second.destroy() }
  } finally { first.destroy() }
})

test('headings, nested lists, tasks, tables and code retain their document structure', () => {
  const markdown = '# Heading\n\n- One\n  - Nested\n\n- [ ] Todo\n- [x] Done\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n```js\nconst x = 1\n```'
  const first = editor(markdown)
  const second = editor(first.getMarkdown())
  try {
    assert.deepEqual(second.getJSON(), first.getJSON())
    const json = JSON.stringify(first.getJSON())
    for (const kind of ['heading', 'bulletList', 'taskList', 'table', 'codeBlock']) assert.ok(json.includes(kind), kind)
  } finally { first.destroy(); second.destroy() }
})

test('images retain Markdown without creating a network-loading image element', () => {
  const value = editor('![Description](https://example.com/private-image.png)')
  try {
    assert.match(value.getMarkdown(), /private-image\.png/)
    assert.doesNotMatch(value.getHTML(), /<img\b/)
    assert.match(value.getHTML(), /Description/)
  } finally { value.destroy() }
})

test('visual slash completion works only in a collapsed paragraph outside code', () => {
  const value = editor('/routine')
  try {
    value.commands.setTextSelection(9)
    const found = richDocumentSlash(value)!
    assert.equal(found.items[0].command, 'routine')
    value.chain().deleteRange({ from: found.from, to: found.to }).insertContent(found.items[0].markdown, { contentType: 'markdown' }).run()
    assert.match(value.getMarkdown(), /## Routine/)
    value.commands.setContent('```\n/doc\n```', { contentType: 'markdown', emitUpdate: false })
    value.commands.setTextSelection(5)
    assert.equal(richDocumentSlash(value), null)
  } finally { value.destroy() }
})

test('opening and externally loading Markdown does not emit a document write', () => {
  const value = editor('**Original**\r\n\r\nBody  ')
  let writes = 0
  value.on('update', () => writes++)
  try {
    value.commands.setContent('Reloaded\n\n- List', { contentType: 'markdown', emitUpdate: false })
    value.commands.setTextSelection(1)
    value.setEditable(false, false)
    value.setEditable(true, false)
    assert.equal(writes, 0)
  } finally { value.destroy() }
})

test('link entry rejects executable and local-file protocols', () => {
  for (const url of ['javascript:alert(1)', 'data:text/html,x', 'file:///C:/secret', 'not a URL']) assert.equal(documentLink(url), null)
  for (const url of ['https://example.com', 'http://localhost/', 'mailto:hello@example.com']) assert.equal(documentLink(url), url)
})
