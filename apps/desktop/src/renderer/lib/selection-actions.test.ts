import assert from 'node:assert/strict'
import { test } from 'node:test'
import { selectionActions, selectionHasLink } from './selection-actions.ts'

test('a selected cell gets Quote, Explain, Improve and Describe', () => {
  const actions = selectionActions('  Revenue 42  ', null)
  assert.deepEqual(actions.map((a) => [a.label, a.icon]), [
    ['Quote', 'text.quote'],
    ['Explain', 'questionmark.circle'],
    ['Improve', 'wand.and.stars'],
    ['Describe', 'photo'],
  ])
  assert.equal(actions[0]!.prompt, '> Revenue 42')
  assert.match(actions[1]!.prompt, /^Explain this:\n\n> Revenue 42$/)
})

test('a selection with a link is recognised and keeps the link in each prompt', () => {
  const text = 'See https://example.com/docs for details'
  assert.equal(selectionHasLink(text), true)
  assert.equal(selectionHasLink('plain words'), false)
  const actions = selectionActions(text, 'thread-1')
  assert.equal(actions.length, 4)
  assert.equal(actions[0]!.thread, 'thread-1')
  assert.match(actions[2]!.prompt, /https:\/\/example\.com\/docs/)
})

test('an empty selection offers nothing and a long one is cut short', () => {
  assert.deepEqual(selectionActions('   \n ', null), [])
  const actions = selectionActions('x'.repeat(3000), null)
  assert.equal(actions[0]!.prompt.length, 2003)
  assert.ok(actions[0]!.prompt.endsWith('…'))
})
