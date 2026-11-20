import { test } from 'node:test'
import assert from 'node:assert/strict'
import { KEYBOARD_SHORTCUTS, shortcutKeys } from './keyboard-shortcuts.ts'

test('every shortcut has a unique id, a label and no chord used twice', () => {
  const ids = new Set<string>()
  const chords = new Set<string>()
  for (const shortcut of KEYBOARD_SHORTCUTS) {
    assert.ok(shortcut.id.trim(), 'a shortcut needs an id')
    assert.equal(ids.has(shortcut.id), false, `duplicate shortcut id ${shortcut.id}`)
    ids.add(shortcut.id)
    assert.ok(shortcut.label.trim(), `${shortcut.id} needs a label`)
    assert.ok(shortcut.keys.trim(), `${shortcut.id} needs keys`)
    for (const chord of shortcut.keys.split('/').map((part) => part.trim())) {
      assert.ok(chord, `${shortcut.id} has an empty chord`)
      assert.equal(chords.has(chord), false, `duplicate chord ${chord}`)
      chords.add(chord)
    }
  }
  for (const id of ['toggleSidebar', 'openSettings', 'openRosterSearch', 'unreadPrevious', 'unreadNext', 'muteSelected']) {
    assert.ok(ids.has(id), `${id} is missing from the cheatsheet`)
  }
})

test('shortcuts stay in non-empty groups and shortcutKeys reads their keys', () => {
  const groups = [...new Set(KEYBOARD_SHORTCUTS.map((shortcut) => shortcut.group))]
  assert.ok(groups.length > 1)
  for (const group of groups) {
    assert.ok(KEYBOARD_SHORTCUTS.some((shortcut) => shortcut.group === group), `${group} is empty`)
  }
  assert.ok(groups.includes('Notifications'))
  for (const shortcut of KEYBOARD_SHORTCUTS) assert.equal(shortcutKeys(shortcut.id), shortcut.keys)
})
