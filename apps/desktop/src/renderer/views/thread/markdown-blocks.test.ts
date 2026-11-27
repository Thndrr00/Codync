import assert from 'node:assert/strict'
import { test } from 'node:test'
import { math_blocks, parseMarkdown } from './markdown-blocks.ts'

test('blocks: headings, nested lists, rules, tables and code', () => {
  const blocks = parseMarkdown('# Title\n- one\n  - two\n1. first\n---\n| A | B |\n|---|---|\n| 1 | 2 |\n```js\nx = 1\n```\n> quoted')
  assert.deepEqual(blocks, [
    { kind: 'heading', text: 'Title', level: 1 },
    { kind: 'bullet', text: 'one', marker: '•', depth: 0 },
    { kind: 'bullet', text: 'two', marker: '◦', depth: 1 },
    { kind: 'bullet', text: 'first', marker: '1.', depth: 0 },
    { kind: 'rule' },
    { kind: 'table', header: ['A', 'B'], rows: [['1', '2']] },
    { kind: 'code', text: 'x = 1', language: 'js' },
    { kind: 'quote', text: 'quoted' },
  ])
})

test('a pipe line without its rule stays text', () => {
  assert.deepEqual(parseMarkdown('| not a table'), [{ kind: 'paragraph', text: '| not a table' }])
})

test('task-list items become checkboxes, with their depth and checked state', () => {
  assert.deepEqual(parseMarkdown('- [x] shipped\n- [ ] next\n  - [X] nested\n- [ ]'), [
    { kind: 'task', text: 'shipped', checked: true, depth: 0 },
    { kind: 'task', text: 'next', checked: false, depth: 0 },
    { kind: 'task', text: 'nested', checked: true, depth: 1 },
    { kind: 'bullet', text: '[ ]', marker: '•', depth: 0 },
  ])
  assert.deepEqual(parseMarkdown('[x] not a list item'), [{ kind: 'paragraph', text: '[x] not a list item' }])
})

test('math: inline and block formulas become escaped source parts', () => {
  assert.deepEqual(math_blocks('Use $a+b$ now.'), [
    { kind: 'text', text: 'Use ' },
    { kind: 'inline', text: 'a+b' },
    { kind: 'text', text: ' now.' },
  ])
  assert.deepEqual(math_blocks('$$E = mc^2$$'), [{ kind: 'block', text: 'E = mc^2' }])
  assert.deepEqual(math_blocks('$$\na+b\n$$'), [{ kind: 'block', text: 'a+b' }])
  assert.deepEqual(math_blocks('before $$x$$ and $y$ after'), [
    { kind: 'text', text: 'before ' },
    { kind: 'block', text: 'x' },
    { kind: 'text', text: ' and ' },
    { kind: 'inline', text: 'y' },
    { kind: 'text', text: ' after' },
  ])
})

test('math: malformed, unclosed and price-like dollar signs stay literal', () => {
  assert.deepEqual(math_blocks('costs $5 and $10'), [{ kind: 'text', text: 'costs $5 and $10' }])
  assert.deepEqual(math_blocks('a $ b'), [{ kind: 'text', text: 'a $ b' }])
  assert.deepEqual(math_blocks('$'), [{ kind: 'text', text: '$' }])
  assert.deepEqual(math_blocks('$$'), [{ kind: 'text', text: '$$' }])
  assert.deepEqual(math_blocks('$a\nb$'), [{ kind: 'text', text: '$a\nb$' }])
  assert.deepEqual(math_blocks('$$a$$ and $$b'), [
    { kind: 'block', text: 'a' },
    { kind: 'text', text: ' and $$b' },
  ])
  assert.deepEqual(math_blocks(`$${'x'.repeat(2_001)}$$`), [{ kind: 'text', text: `$${'x'.repeat(2_001)}$$` }])
})

test('math: a block formula on its own lines is one block, inline math stays in its paragraph', () => {
  assert.deepEqual(parseMarkdown('Text\n$$\na+b\n$$\nAfter'), [
    { kind: 'paragraph', text: 'Text' },
    { kind: 'math', text: 'a+b' },
    { kind: 'paragraph', text: 'After' },
  ])
  assert.deepEqual(parseMarkdown('$x$ inline'), [{ kind: 'paragraph', text: '$x$ inline' }])
  assert.deepEqual(parseMarkdown('$$\nunclosed'), [{ kind: 'paragraph', text: '$$\nunclosed' }])
})
