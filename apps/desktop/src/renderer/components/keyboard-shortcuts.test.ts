import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  captureChord, chordKey, chordLabel, DEFAULT_BINDINGS, isReservedChord, isValidChord, KEYBOARD_SHORTCUTS,
  matchesChord, normalizeKey, REBINDABLE_SHORTCUTS, resolveBindings, sanitizeBindings, shortcutForEvent,
  shortcutKeys, type Chord, type ChordEvent,
} from './keyboard-shortcuts.ts'

const event = (key: string, mods: Partial<ChordEvent> = {}): ChordEvent => ({
  key, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods,
})

test('every shortcut has a unique id, a label and keys, and no default chord used twice', () => {
  const ids = new Set<string>()
  const chords = new Set<string>()
  for (const shortcut of KEYBOARD_SHORTCUTS) {
    assert.ok(shortcut.id.trim(), 'a shortcut needs an id')
    assert.equal(ids.has(shortcut.id), false, `duplicate shortcut id ${shortcut.id}`)
    ids.add(shortcut.id)
    assert.ok(shortcut.label.trim(), `${shortcut.id} needs a label`)
    assert.ok(shortcut.keys.trim(), `${shortcut.id} needs keys`)
    if (shortcut.chord) {
      assert.ok(isValidChord(shortcut.chord))
      assert.equal(isReservedChord(shortcut.chord), false, `${shortcut.id} defaults to a reserved chord`)
      const signature = chordKey(shortcut.chord)
      assert.equal(chords.has(signature), false, `duplicate default chord ${signature}`)
      chords.add(signature)
    }
  }
  for (const id of ['toggleSidebar', 'openSettings', 'openRosterSearch', 'unreadPrevious', 'unreadNext', 'muteSelected', 'findInChat', 'addAgent']) {
    assert.ok(ids.has(id), `${id} is missing from the cheatsheet`)
  }
  for (const id of Object.keys(DEFAULT_BINDINGS)) assert.ok(ids.has(id), `${id} has a binding but no row`)
  for (const shortcut of REBINDABLE_SHORTCUTS) assert.equal(DEFAULT_BINDINGS[shortcut.id], shortcut.chord)
})

test('shortcuts stay in non-empty groups and shortcutKeys reads their keys', () => {
  const groups = [...new Set(KEYBOARD_SHORTCUTS.map((shortcut) => shortcut.group))]
  assert.ok(groups.length > 1)
  for (const group of groups) {
    assert.ok(KEYBOARD_SHORTCUTS.some((shortcut) => shortcut.group === group), `${group} is empty`)
  }
  assert.ok(groups.includes('Notifications'))
  for (const shortcut of KEYBOARD_SHORTCUTS) {
    assert.ok(shortcutKeys(shortcut.id), `${shortcut.id} has no display keys`)
    if (!shortcut.rebindable) assert.equal(shortcutKeys(shortcut.id), shortcut.keys)
  }
})

test('a chord key and label name modifiers in a fixed order on each platform', () => {
  const chord: Chord = { mod: true, alt: false, shift: true, key: 'o' }
  assert.equal(chordKey(chord), 'mod+shift+o')
  assert.equal(normalizeKey('O'), 'o')
  assert.equal(normalizeKey(' '), 'Space')
  assert.equal(chordLabel(chord, 'darwin'), '⌘⇧O')
  assert.equal(chordLabel(chord, 'win32'), 'Ctrl+Shift+O')
  assert.equal(chordLabel({ mod: false, alt: false, shift: false, key: '?' }, 'darwin'), '?')
})

test('capture cancels on Escape, refuses lone modifiers and reserved chords', () => {
  const taken = Object.values(DEFAULT_BINDINGS)
  assert.deepEqual(captureChord(event('Escape', { metaKey: true, shiftKey: true }), taken), { ok: false, reason: 'cancelled' })
  for (const key of ['Shift', 'Control', 'Alt', 'Meta']) {
    assert.deepEqual(captureChord(event(key, { shiftKey: key === 'Shift' }), taken), { ok: false, reason: 'modifier' })
  }
  assert.deepEqual(captureChord(event('Enter'), taken), { ok: false, reason: 'reserved' })
  assert.deepEqual(captureChord(event('Tab', { shiftKey: true }), taken), { ok: false, reason: 'reserved' })
  assert.deepEqual(captureChord(event('a', { isComposing: true }), taken), { ok: false, reason: 'empty' })
})

test('capture refuses a chord another row owns and accepts the same chord it already has', () => {
  const current = DEFAULT_BINDINGS.commandPalette!
  assert.deepEqual(captureChord(event('k', { metaKey: true }), [current]), { ok: false, reason: 'duplicate' })
  assert.deepEqual(captureChord(event('K', { ctrlKey: true }), [current]), { ok: false, reason: 'duplicate' })
  assert.deepEqual(captureChord(event('k', { metaKey: true }), []), { ok: true, chord: current })
  assert.deepEqual(captureChord(event('ArrowDown', { altKey: true, shiftKey: true }), []), {
    ok: true, chord: { mod: false, alt: true, shift: true, key: 'ArrowDown' },
  })
  assert.deepEqual(captureChord(event('?', { shiftKey: true }), []), { ok: true, chord: { mod: false, alt: false, shift: true, key: '?' } })
})

test('resolution applies valid overrides and drops malformed, reserved and duplicate ones', () => {
  const override: Chord = { mod: true, alt: true, shift: false, key: 'y' }
  assert.deepEqual(resolveBindings({ commandPalette: override }).commandPalette, override)
  const defaults = resolveBindings({})
  assert.deepEqual(defaults, DEFAULT_BINDINGS, 'no stored overrides means the built-in bindings')

  assert.deepEqual(resolveBindings({ commandPalette: { mod: true, alt: false, shift: false } }).commandPalette, DEFAULT_BINDINGS.commandPalette)
  assert.deepEqual(resolveBindings({ commandPalette: { mod: true, alt: false, shift: false, key: 'Shift' } }).commandPalette, DEFAULT_BINDINGS.commandPalette)
  assert.deepEqual(resolveBindings({ commandPalette: { mod: false, alt: false, shift: false, key: 'Escape' } }).commandPalette, DEFAULT_BINDINGS.commandPalette)
  assert.deepEqual(resolveBindings({ commandPalette: DEFAULT_BINDINGS.toggleSidebar }).commandPalette, DEFAULT_BINDINGS.commandPalette)
  assert.deepEqual(resolveBindings({ commandPalette: override, findInChat: override }).findInChat, DEFAULT_BINDINGS.findInChat)
  assert.deepEqual(resolveBindings({ commandPalette: override }).findInChat, DEFAULT_BINDINGS.findInChat)
  assert.deepEqual(resolveBindings(null), DEFAULT_BINDINGS)
  assert.deepEqual(resolveBindings([]), DEFAULT_BINDINGS)
})

test('a stored junk map resolves to defaults, and clearing overrides resets every row', () => {
  const custom = resolveBindings({ commandPalette: { mod: true, alt: false, shift: false, key: 'y' } })
  assert.notDeepEqual(custom, DEFAULT_BINDINGS)
  assert.deepEqual(sanitizeBindings(custom), custom, 'a valid saved map survives validation')
  assert.deepEqual(sanitizeBindings('{"commandPalette":42}'), DEFAULT_BINDINGS)
  assert.deepEqual(resolveBindings({}), DEFAULT_BINDINGS, 'Reset all restores the built-in chords')
})

test('a key event matches only its bound chord, and typing keeps the other shortcuts out', () => {
  assert.ok(matchesChord(event('K', { metaKey: true }), DEFAULT_BINDINGS.commandPalette!))
  assert.equal(matchesChord(event('k', { metaKey: true, shiftKey: true }), DEFAULT_BINDINGS.commandPalette!), false)
  assert.ok(matchesChord(event('k', { ctrlKey: true }), DEFAULT_BINDINGS.commandPalette!), 'mod is ⌘ on macOS and Ctrl elsewhere')
  assert.equal(matchesChord(event('k'), DEFAULT_BINDINGS.commandPalette!), false)
  assert.equal(matchesChord(event('k', { metaKey: true, isComposing: true }), DEFAULT_BINDINGS.commandPalette!), false)
  assert.ok(matchesChord(event('?', { shiftKey: true }), { mod: false, alt: false, shift: false, key: '?' }), 'the ? character carries its own Shift')
  assert.equal(matchesChord(event('?', { altKey: true }), { mod: false, alt: false, shift: false, key: '?' }), false)

  assert.equal(shortcutForEvent(event('k', { metaKey: true }), DEFAULT_BINDINGS), 'commandPalette')
  assert.equal(shortcutForEvent(event('k', { metaKey: true }), DEFAULT_BINDINGS, true), 'commandPalette')
  assert.equal(shortcutForEvent(event('b', { metaKey: true }), DEFAULT_BINDINGS, true), undefined)
  assert.equal(shortcutForEvent(event('b', { metaKey: true }), DEFAULT_BINDINGS), 'toggleSidebar')
  const custom = resolveBindings({ toggleSidebar: { mod: true, alt: false, shift: false, key: 'y' } })
  assert.equal(shortcutForEvent(event('b', { metaKey: true }), custom), undefined, 'the old chord no longer fires')
  assert.equal(shortcutForEvent(event('y', { metaKey: true }), custom), 'toggleSidebar')
})

test('shortcut hints read the effective binding, not the default', () => {
  const custom = resolveBindings({ commandPalette: { mod: true, alt: false, shift: false, key: 'y' } })
  assert.equal(shortcutKeys('commandPalette', custom, 'darwin'), '⌘Y')
  assert.equal(shortcutKeys('commandPalette', custom, 'win32'), 'Ctrl+Y')
  assert.equal(shortcutKeys('commandPalette', undefined, 'darwin'), '⌘K')
  assert.equal(shortcutKeys('keyboardShortcuts', undefined, 'darwin'), '⌘/ (?)', 'the ? alias stays in the hint')
  assert.equal(shortcutKeys('keyboardShortcuts', resolveBindings({ keyboardShortcuts: { mod: false, alt: false, shift: false, key: 'p' } }), 'darwin'), 'P')
  assert.equal(shortcutKeys('closeOverlay', custom, 'darwin'), 'Esc', 'fixed keys never change')
  assert.equal(shortcutKeys('missing', custom), undefined)
})
