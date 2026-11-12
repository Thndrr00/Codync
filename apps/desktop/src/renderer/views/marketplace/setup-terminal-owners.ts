/** A live setup screen still owns its process. Revealing it must not unmount and terminate it. */
const owners = new WeakMap<object, Map<string, () => void>>()

export function ownSetupTerminal(store: object, id: string, reveal: () => void) {
  const entries = owners.get(store) ?? new Map<string, () => void>()
  entries.set(id, reveal); owners.set(store, entries)
  return () => { if (entries.get(id) === reveal) entries.delete(id) }
}

export function revealOwnedSetupTerminal(store: object, id: string): boolean {
  const reveal = owners.get(store)?.get(id)
  if (!reveal) return false
  reveal()
  return true
}
