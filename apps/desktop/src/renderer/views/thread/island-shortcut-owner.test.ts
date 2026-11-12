import assert from 'node:assert/strict'
import { test } from 'node:test'
import { IslandShortcuts, type IslandShortcutRegistry } from '../../../main/island-shortcuts.ts'
import { islandAcceleratorKey, type IslandShortcutAction } from '../../../shared/island-shortcuts.ts'

function fixture() {
  const registrations = new Map<string, () => void>()
  const external = new Set<string>()
  const unavailable = new Set<string>()
  const registrationErrors = new Set<string>()
  const releaseErrors = new Set<string>()
  const registered: string[] = []
  const unregistered: string[] = []
  const invoked: IslandShortcutAction[] = []
  const registry: IslandShortcutRegistry = {
    isRegistered: (accelerator) => registrations.has(islandAcceleratorKey(accelerator)) || external.has(islandAcceleratorKey(accelerator)),
    register(accelerator, callback) {
      registered.push(accelerator)
      if (registrationErrors.has(accelerator)) throw new Error('registry failure')
      if (unavailable.has(accelerator)) return false
      registrations.set(islandAcceleratorKey(accelerator), callback)
      return true
    },
    unregister(accelerator) {
      unregistered.push(accelerator)
      if (releaseErrors.has(accelerator)) throw new Error('release failure')
      registrations.delete(islandAcceleratorKey(accelerator))
    },
  }
  const owner = new IslandShortcuts(registry, (action) => invoked.push(action))
  const callback = (accelerator: string) => {
    const found = registrations.get(islandAcceleratorKey(accelerator))
    assert.ok(found, `expected owned registration for ${accelerator}`)
    return found
  }
  return { owner, callback, external, registrations, unavailable, registrationErrors, releaseErrors, registered, unregistered, invoked }
}

test('inactive settings do not acquire keys and deactivation releases only island-owned registrations', () => {
  const f = fixture()
  f.external.add(islandAcceleratorKey('Alt+L'))
  f.owner.set({ toggle: 'Alt+J', search: 'Control+Alt+K' })
  assert.deepEqual(f.registered, [])
  f.owner.activate()
  assert.deepEqual(f.registered, ['Alt+J', 'Control+Alt+K'])
  f.callback('Alt+J')()
  assert.deepEqual(f.invoked, ['toggle'])
  f.owner.deactivate()
  assert.deepEqual(f.unregistered, ['Alt+J', 'Control+Alt+K'])
  assert.equal(f.external.has(islandAcceleratorKey('Alt+L')), true)
  assert.equal(f.registrations.size, 0)
  f.owner.deactivate()
  assert.equal(f.unregistered.length, 2)
})

test('conflicts, failed registrations and registry exceptions remain visible without taking or releasing foreign keys', () => {
  const f = fixture()
  f.external.add(islandAcceleratorKey('Alt+L'))
  f.unavailable.add('Alt+U')
  f.registrationErrors.add('Alt+A')
  f.owner.set({ toggle: 'Alt+J', search: 'Alt+L', activity: 'Alt+A', usage: 'Alt+U' })
  const state = f.owner.activate()
  assert.match(state.errors.search, /already registered/i)
  assert.match(state.errors.activity, /registry failure/i)
  assert.match(state.errors.usage, /unavailable/i)
  assert.equal(state.errors.toggle, undefined)
  assert.equal(f.registered.includes('Alt+L'), false)
  f.owner.deactivate()
  assert.deepEqual(f.unregistered, ['Alt+J'])
  assert.equal(f.external.has(islandAcceleratorKey('Alt+L')), true)
})

test('duplicate and reserved bindings are rejected before the registry receives them', () => {
  const f = fixture()
  f.owner.set({ toggle: 'Control+Alt+K', search: 'Option+Ctrl+K', message: 'Alt+Y', quiet: 'Command+Q' })
  const state = f.owner.activate()
  assert.match(state.errors.toggle, /duplicate/i)
  assert.match(state.errors.search, /duplicate/i)
  assert.match(state.errors.message, /reserved/i)
  assert.match(state.errors.quiet, /reserved/i)
  assert.deepEqual(f.registered, [])
  f.owner.deactivate()
  assert.deepEqual(f.unregistered, [])
})

test('replaced registrations cannot invoke stale callbacks even when the same action keeps the same accelerator', () => {
  const f = fixture()
  f.owner.set({ toggle: 'Alt+J' })
  f.owner.activate()
  const first = f.callback('Alt+J')
  f.owner.set({ search: 'Alt+J' })
  const second = f.callback('Alt+J')
  first()
  assert.deepEqual(f.invoked, [])
  second()
  assert.deepEqual(f.invoked, ['search'])
  f.owner.set({ search: 'Alt+J' })
  const third = f.callback('Alt+J')
  second()
  assert.deepEqual(f.invoked, ['search'])
  third()
  assert.deepEqual(f.invoked, ['search', 'search'])
  f.owner.deactivate()
  third()
  assert.deepEqual(f.invoked, ['search', 'search'])
  f.owner.activate()
  third()
  assert.deepEqual(f.invoked, ['search', 'search'])
  f.callback('Alt+J')()
  assert.deepEqual(f.invoked, ['search', 'search', 'search'])
})

test('a failed release retains ownership for retry while inactive callbacks remain disabled', () => {
  const f = fixture()
  f.owner.set({ toggle: 'Alt+J' })
  f.owner.activate()
  const callback = f.callback('Alt+J')
  f.releaseErrors.add('Alt+J')
  assert.match(f.owner.deactivate().errors.toggle, /release failure/i)
  callback()
  assert.deepEqual(f.invoked, [])
  assert.equal(f.registrations.size, 1)
  f.releaseErrors.clear()
  f.owner.deactivate()
  assert.deepEqual(f.unregistered, ['Alt+J', 'Alt+J'])
  assert.equal(f.registrations.size, 0)
})

test('settings inputs and returned bindings or errors cannot mutate internal registration state', () => {
  const f = fixture()
  const input = { toggle: 'Alt+J', search: 'Alt+Y' }
  const state = f.owner.set(input)
  input.toggle = 'Alt+L'
  state.bindings.toggle = 'Alt+K'
  state.errors.search = 'changed externally'
  assert.equal(f.owner.state.bindings.toggle, 'Alt+J')
  assert.match(f.owner.state.errors.search, /reserved/i)
  f.owner.activate()
  const snapshot = f.owner.state
  snapshot.bindings.toggle = ''
  snapshot.errors.toggle = 'invented error'
  f.callback('Alt+J')()
  assert.deepEqual(f.invoked, ['toggle'])
  assert.equal(f.owner.state.errors.toggle, undefined)
})
