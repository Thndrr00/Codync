import assert from 'node:assert/strict'
import { test } from 'node:test'
import { setMaxListeners } from 'node:events'

test('A stored preference of another kind reads as the fallback', async () => {
  const stored = new Map<string, string>([
    ['hiddenComputers', 'null'],
    ['computerOrder', '{"a":1}'],
    ['sidebarCompact', '"yes"'],
    ['compactMessageActions', 'true'],
    ['desktopSidebarWidth', '300'],
    ['computerAccessSetup', '"ready"'],
    ['accentColor', '"#1084FE"'],
    ['restoreLastWorkspace', '"yes"'],
    ['startupAnimation', '"yes"'],
    ['openTabsByChat', '[]'],
  ])
  const events = new EventTarget()
  setMaxListeners(0, events)
  Object.defineProperty(globalThis, 'window', { configurable: true, value: events })
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: { getItem: (key: string) => stored.get(key) ?? null, setItem: (key: string, value: string) => void stored.set(key, value) },
  })
  const { prefs } = await import('./prefs.ts')
  assert.equal(prefs.hiddenComputers.get(), '')
  assert.deepEqual(prefs.computerOrder.get(), [])
  assert.equal(prefs.sidebarCompact.get(), false)
  assert.equal(prefs.compactActions.get(), true, 'a stored compact-actions switch is read as a boolean')
  assert.equal(prefs.sidebarWidth.get(), 300)
  assert.equal(prefs.computerAccessSetup.get(), 'ready')
  assert.equal(prefs.accentColor.get(), '#1084FE')
  assert.equal(prefs.restoreLastWorkspace.get(), true, 'a non-boolean switch reads as its default')
  assert.equal(prefs.startupAnimation.get(), true, 'a non-boolean startup switch reads as its default')
  assert.deepEqual(prefs.openTabsByChat.get(), {}, 'an array is not a tab map')
})

test('a custom accent writes the accent variables and clearing it removes them', async () => {
  const props = new Map<string, string>()
  const style = {
    setProperty: (name: string, value: string) => void props.set(name, value),
    removeProperty: (name: string) => void props.delete(name),
  }
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { documentElement: { style } },
  })
  const { applyAccentColor } = await import('./prefs.ts')
  applyAccentColor('#2B2B2B')
  assert.equal(props.get('--accent'), '#2B2B2B')
  assert.equal(props.get('--accent-fill'), '#2B2B2B')
  assert.equal(props.get('--on-accent'), '#ffffff', 'white ink reads on a dark fill')
  applyAccentColor('#F8F8F8')
  assert.equal(props.get('--on-accent'), '#141414', 'near-black ink reads on a near-white fill')
  applyAccentColor(null)
  assert.equal(props.has('--accent'), false)
  assert.equal(props.has('--accent-fill'), false)
  assert.equal(props.has('--on-accent'), false)
})
