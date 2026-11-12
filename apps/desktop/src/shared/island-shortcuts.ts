export type IslandShortcutAction = 'toggle' | 'search' | 'activity' | 'message' | 'usage' | 'quiet'
export type IslandShortcutBindings = Record<IslandShortcutAction, string>

export interface IslandShortcutState {
  bindings: IslandShortcutBindings
  errors: Record<string, string>
}

export const islandShortcutActions: readonly IslandShortcutAction[] = ['toggle', 'search', 'activity', 'message', 'usage', 'quiet']

/** Global shortcuts are opt-in; enabling the island never takes a new key combination. */
export const defaultIslandBindings: IslandShortcutBindings = {
  toggle: '', search: '', activity: '', message: '', usage: '', quiet: '',
}

export function normalizeBindings(input: unknown): IslandShortcutBindings {
  const value = input !== null && typeof input === 'object' && !Array.isArray(input) ? input as Record<string, unknown> : {}
  const bindings = { ...defaultIslandBindings }
  for (const action of islandShortcutActions) {
    const binding = Object.hasOwn(value, action) ? value[action] : null
    bindings[action] = typeof binding === 'string' ? binding.trim().slice(0, 80) : ''
  }
  return bindings
}

/** Electron accepts modifier aliases and different ordering; compare their meaning, not spelling. */
export function islandAcceleratorKey(accelerator: string): string {
  const aliases: Record<string, string> = {
    commandorcontrol: 'cmdorctrl', commandorctrl: 'cmdorctrl', cmdorcontrol: 'cmdorctrl',
    command: 'cmd', control: 'ctrl', option: 'alt', meta: 'super',
  }
  const parts = accelerator.replace(/\s+/g, '').toLowerCase().replace(/\+\+$/, '+plus').split('+')
  const key = parts.pop() ?? ''
  const modifiers = [...new Set(parts.map((part) => aliases[part] ?? part))].sort()
  return [...modifiers, key].join('+')
}

/** Keep approval, editing, app-navigation and fullscreen controls out of the global registry. */
export function reservedIslandAccelerator(accelerator: string): boolean {
  const key = islandAcceleratorKey(accelerator)
  if (['alt+y', 'alt+n', 'alt+f4', 'f11'].includes(key)) return true
  const parts = key.split('+')
  const letter = parts.pop() ?? ''
  const command = parts.some((part) => ['cmdorctrl', 'ctrl', 'cmd', 'super'].includes(part))
  if (!command) return false
  if (['a', 'c', 'v', 'x', 'z', 'y', 'q', 'w', 'm', 'h', 'o', 'p', 'n', 'f', 'r', 's', '=', 'plus', '-', 'minus', '0'].includes(letter)) return true
  return letter === 'i' && (parts.includes('alt') || parts.includes('shift'))
}

export function islandBindingErrors(bindings: IslandShortcutBindings): Record<string, string> {
  const errors: Record<string, string> = {}
  const groups = new Map<string, IslandShortcutAction[]>()
  for (const action of islandShortcutActions) {
    const binding = bindings[action]
    if (!binding) continue
    if (reservedIslandAccelerator(binding)) errors[action] = 'This shortcut is reserved by Codync or the system.'
    const key = islandAcceleratorKey(binding)
    groups.set(key, [...(groups.get(key) ?? []), action])
  }
  for (const actions of groups.values()) {
    if (actions.length < 2) continue
    for (const action of actions) errors[action] = 'Duplicate shortcut: choose a different binding.'
  }
  return errors
}
