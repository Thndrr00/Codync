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
    ['interfaceFont', '"red; } body { color: blue"'],
    ['codeFont', '123'],
    ['restoreLastWorkspace', '"yes"'],
    ['startupAnimation', '"yes"'],
    ['openTabsByChat', '[]'],
    ['openWorkflowTabs', '{"a":1}'],
    ['keyboardBindings', '{"commandPalette":{"mod":true,"alt":false,"shift":false,"key":"Shift"},"toggleSidebar":{"mod":true,"alt":false,"shift":false,"key":"y"},"unknown":{"mod":true,"alt":false,"shift":false,"key":"z"}}'],
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
  assert.equal(prefs.interfaceFont.get(), null, 'a font name with CSS syntax reads as the default')
  assert.equal(prefs.codeFont.get(), null, 'a non-string font reads as the default')
  assert.equal(prefs.restoreLastWorkspace.get(), true, 'a non-boolean switch reads as its default')
  assert.equal(prefs.startupAnimation.get(), true, 'a non-boolean startup switch reads as its default')
  assert.deepEqual(prefs.openTabsByChat.get(), {}, 'an array is not a tab map')
  assert.deepEqual(prefs.openWorkflowTabs.get(), [], 'an object is not a workflow tab list')
  const bindings = prefs.keyboardBindings.get()
  assert.equal(bindings.commandPalette?.key, 'k', 'a lone modifier is not a usable binding')
  assert.equal(bindings.toggleSidebar?.key, 'y', 'a well-formed override is kept')
  assert.equal(Object.hasOwn(bindings, 'unknown'), false, 'an unknown shortcut id is dropped')
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

test('a chosen font family prefixes the trusted stack and never injects CSS', async () => {
  const { font_stack } = await import('./prefs.ts')
  const fallback = 'system-ui, sans-serif'
  assert.equal(font_stack('Inter', fallback), 'Inter, system-ui, sans-serif')
  assert.equal(font_stack('SF Mono', fallback), '"SF Mono", system-ui, sans-serif')
  assert.equal(font_stack('Noto Sans CJK TC', fallback), '"Noto Sans CJK TC", system-ui, sans-serif')
  for (const bad of ['red;}body{color:blue', 'a, b', 'url(evil)', '"quoted"', 'a\\b', null]) {
    assert.equal(font_stack(bad, fallback), fallback, `rejected: ${bad}`)
  }
})

test('font choices write the two root variables and clearing them restores the theme', async () => {
  const props = new Map<string, string>()
  const style = {
    setProperty: (name: string, value: string) => void props.set(name, value),
    removeProperty: (name: string) => void props.delete(name),
  }
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { documentElement: { style } },
  })
  const { applyFonts } = await import('./prefs.ts')
  applyFonts('Inter', 'JetBrains Mono')
  assert.match(props.get('--font') ?? '', /^Inter, /)
  assert.match(props.get('--mono') ?? '', /^"JetBrains Mono", /)
  applyFonts(null, null)
  assert.equal(props.has('--font'), false)
  assert.equal(props.has('--mono'), false)
})
