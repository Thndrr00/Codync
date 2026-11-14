import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CHAT_BACKGROUNDS, CHAT_COLOURS, chatChoiceId, cycleChatChoice, decodeChatChoice, encodeChatChoice, mixHex } from './chat-appearance.ts'

const ids = CHAT_COLOURS.map((preset) => preset.id)

test('a chat colour saved before the themes split applies to both', () => {
  const choice = decodeChatChoice('green', ids)
  assert.deepEqual(choice, { light: 'green', dark: 'green' })
  assert.equal(encodeChatChoice(choice), 'green/green')
})

test('each theme keeps its own choice and a dash is none', () => {
  assert.deepEqual(decodeChatChoice('red/-', ids), { light: 'red', dark: null })
  assert.deepEqual(decodeChatChoice('-/blue', ids), { light: null, dark: 'blue' })
  assert.deepEqual(decodeChatChoice('', ids), { light: null, dark: null })
  assert.deepEqual(decodeChatChoice('purple', ids), { light: null, dark: null })
  assert.equal(chatChoiceId({ light: 'red', dark: 'blue' }, true), 'blue')
})

test('cycling walks the presets then returns to none', () => {
  let choice = { light: null as string | null, dark: null as string | null }
  for (const expected of [...ids, null]) {
    choice = cycleChatChoice(choice, false, ids)
    assert.equal(choice.light, expected)
  }
  assert.equal(choice.dark, null)
})

test('the eleven presets have a light and a dark shade each', () => {
  assert.equal(CHAT_COLOURS.length, 11)
  assert.equal(CHAT_BACKGROUNDS.length, 11)
  for (const preset of CHAT_COLOURS) {
    assert.notEqual(preset.light, preset.dark)
    assert.ok(CHAT_BACKGROUNDS.some((background) => background.id === preset.id))
  }
})

test('a bubble tint mixes the theme base with the preset', () => {
  assert.equal(mixHex('#E2E2E2', '#000000', 0), '#e2e2e2')
  assert.equal(mixHex('#E2E2E2', '#000000', 1), '#000000')
  assert.equal(mixHex('#000000', '#ffffff', 0.5), '#808080')
})
