import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  defaultIslandBindings, islandAcceleratorKey, islandBindingErrors, normalizeBindings, reservedIslandAccelerator,
} from './island-shortcuts.ts'

test('global island shortcuts are opt-in and normalize only owned string settings', () => {
  for (const input of [null, [], 42, 'Control+Alt+K']) assert.deepEqual(normalizeBindings(input), defaultIslandBindings)
  const inherited = Object.assign(Object.create({ toggle: 'Alt+J' }), { search: '  Control+Alt+K  ', usage: 42, quiet: 'x'.repeat(100) })
  assert.deepEqual(normalizeBindings(inherited), { ...defaultIslandBindings, search: 'Control+Alt+K', quiet: 'x'.repeat(80) })
  const input = { toggle: 'Alt+J', unexpected: 'Alt+K' }
  const normalized = normalizeBindings(input)
  input.toggle = 'Alt+L'
  assert.equal(normalized.toggle, 'Alt+J')
  assert.equal(Object.hasOwn(normalized, 'unexpected'), false)
  normalized.activity = 'Alt+L'
  assert.equal(defaultIslandBindings.activity, '')
})

test('accelerator aliases, modifier order, duplicate modifiers and plus spelling compare consistently', () => {
  const equivalent = [
    ['Control+Alt+K', ' Option + CTRL + k '],
    ['CommandOrControl+Shift+K', 'shift+CmdOrCtrl+k'],
    ['CmdOrControl+K', 'CommandOrCtrl+K'],
    ['Command+Alt+K', 'Option+Cmd+K'],
    ['Meta+K', 'Super+K'],
    ['Ctrl+Ctrl+Alt+K', 'Alt+Control+K'],
    ['Control++', 'Ctrl+Plus'],
  ]
  for (const [first, second] of equivalent) assert.equal(islandAcceleratorKey(first), islandAcceleratorKey(second))
  assert.notEqual(islandAcceleratorKey('Control+Alt+K'), islandAcceleratorKey('Control+Shift+K'))
})

test('approval, editing, app navigation and system shortcuts cannot become global island bindings', () => {
  for (const accelerator of [
    'Alt+Y', 'Option+N', 'Alt+F4', 'F11', 'Control+C', 'Command+Q', 'CmdOrCtrl+Shift+S',
    'Command+Alt+I', 'Control+Shift+I', 'Control++', 'CmdOrCtrl+-', 'Meta+W',
  ]) assert.equal(reservedIslandAccelerator(accelerator), true, accelerator)
  for (const accelerator of ['Alt+J', 'Control+Alt+K', 'CmdOrCtrl+Shift+U']) {
    assert.equal(reservedIslandAccelerator(accelerator), false, accelerator)
  }
})

test('every action in a duplicate alias group gets an error without hiding unrelated reservations', () => {
  const bindings = normalizeBindings({ toggle: 'Control+Alt+K', search: 'Option+Ctrl+K', activity: 'Alt+L', quiet: 'Alt+Y' })
  const errors = islandBindingErrors(bindings)
  assert.match(errors.toggle, /duplicate/i)
  assert.match(errors.search, /duplicate/i)
  assert.match(errors.quiet, /reserved/i)
  assert.equal(errors.activity, undefined)
  assert.deepEqual(bindings, { ...defaultIslandBindings, toggle: 'Control+Alt+K', search: 'Option+Ctrl+K', activity: 'Alt+L', quiet: 'Alt+Y' })
})
