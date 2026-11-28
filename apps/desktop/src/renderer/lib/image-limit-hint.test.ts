import assert from 'node:assert/strict'
import { test } from 'node:test'
import { image_limit_hint } from './image-limit-hint.ts'

test('an image refusal gets one bounded, actionable hint', () => {
  const hint = image_limit_hint('This model refuses to continue: too many images in this conversation.')
  assert.equal(hint, 'Too many images. Start a new chat and ask the agent to open fewer images per step, or choose a model that reads files.')
  assert.ok(hint !== null && hint.length <= 160)
})

test('other errors and empty text get no hint', () => {
  assert.equal(image_limit_hint('Rate limited. Try again soon.'), null)
  assert.equal(image_limit_hint('Could not read that file.'), null)
  assert.equal(image_limit_hint('  '), null)
  assert.equal(image_limit_hint(''), null)
})

test('the hint is the same for every wording and never echoes the error', () => {
  const first = image_limit_hint('too many images')
  const long = image_limit_hint('too many images '.repeat(200) + 'and a very long tail')
  assert.equal(first, long)
  assert.ok(!long?.includes('and a very long tail'))
})
