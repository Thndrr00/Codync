import {
  defaultIslandBindings, islandAcceleratorKey, islandBindingErrors, islandShortcutActions, normalizeBindings,
  type IslandShortcutAction, type IslandShortcutBindings, type IslandShortcutState,
} from '../shared/island-shortcuts.ts'

export interface IslandShortcutRegistry {
  register(accelerator: string, callback: () => void): boolean
  unregister(accelerator: string): void
  isRegistered(accelerator: string): boolean
}

/** Own only this island's registrations; changing settings must never remove another feature's keys. */
export class IslandShortcuts {
  private bindings: IslandShortcutBindings = { ...defaultIslandBindings }
  private errors: Record<string, string> = {}
  private active = false
  private owned = new Map<string, { accelerator: string; action: IslandShortcutAction }>()

  private generation = 0
  private readonly registry: IslandShortcutRegistry
  private readonly invoke: (action: IslandShortcutAction) => void
  constructor(registry: IslandShortcutRegistry, invoke: (action: IslandShortcutAction) => void) {
    this.registry = registry
    this.invoke = invoke
  }

  get state(): IslandShortcutState {
    return { bindings: { ...this.bindings }, errors: { ...this.errors } }
  }

  set(input: unknown): IslandShortcutState {
    const generation = ++this.generation
    this.bindings = normalizeBindings(input)
    this.errors = islandBindingErrors(this.bindings)
    this.release()
    if (!this.active) return this.state
    for (const action of islandShortcutActions) {
      const accelerator = this.bindings[action]
      if (!accelerator || this.errors[action]) continue
      const key = islandAcceleratorKey(accelerator)
      try {
        if (this.registry.isRegistered(accelerator)) {
          this.errors[action] = 'Shortcut is already registered.'
          continue
        }
        const registered = this.registry.register(accelerator, () => {
          if (generation === this.generation && this.active && this.owned.get(key)?.action === action && islandAcceleratorKey(this.bindings[action]) === key) this.invoke(action)
        })
        if (registered) this.owned.set(key, { accelerator, action })
        else this.errors[action] = 'Shortcut is unavailable or used by another app.'
      } catch (error) {
        this.errors[action] = `Could not register shortcut: ${error instanceof Error ? error.message : String(error)}`
      }
    }
    return this.state
  }

  activate(): IslandShortcutState {
    this.active = true
    return this.set(this.bindings)
  }

  deactivate(): IslandShortcutState {
    this.active = false
    ++this.generation
    this.release()
    return this.state
  }

  private release() {
    for (const [key, { accelerator, action }] of this.owned) {
      try {
        this.registry.unregister(accelerator)
        this.owned.delete(key)
      } catch (error) {
        // Retain ownership so a later deactivate retries; inactive/stale callbacks cannot invoke actions.
        this.errors[action] = `Could not release shortcut: ${error instanceof Error ? error.message : String(error)}`
      }
    }
  }
}
