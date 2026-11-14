// Where a computer's mirror cache lives: the encrypted file in the main process, or this window's
// storage when the bridge has none (the demo). A plaintext cache from an older build is moved into
// the encrypted one the first time it is read, then removed.

export interface MirrorCacheBackend {
  read(key: string): Promise<string | null>
  write(key: string, text: string): Promise<void>
  remove(key: string): Promise<void>
}

export class MirrorCache {
  private readonly backend: MirrorCacheBackend
  /** The window's storage, read once to migrate a cache an older build left there. */
  private readonly legacy: Storage | null

  constructor(backend: MirrorCacheBackend, legacy: Storage | null = null) {
    this.backend = backend
    this.legacy = legacy
  }

  async read(key: string): Promise<string | null> {
    try {
      const saved = await this.backend.read(key)
      if (saved !== null) return saved
    } catch (error) {
      console.warn('Could not read the mirror cache', error)
    }
    let old: string | null = null
    try {
      old = this.legacy?.getItem(key) ?? null
    } catch {}
    if (old === null) return null
    try {
      await this.backend.write(key, old)
      this.legacy?.removeItem(key)
    } catch {}
    return old
  }

  async write(key: string, text: string): Promise<void> {
    try {
      await this.backend.write(key, text)
      this.legacy?.removeItem(key)
    } catch (error) {
      console.warn('Could not write the mirror cache', error)
    }
  }

  async remove(key: string): Promise<void> {
    try {
      await this.backend.remove(key)
    } catch {}
    try {
      this.legacy?.removeItem(key)
    } catch {}
  }
}

/** Storage as a backend: the fallback when the window has no encrypted cache (the demo). */
export function storageBackend(storage: Storage): MirrorCacheBackend {
  return {
    async read(key) {
      return storage.getItem(key)
    },
    async write(key, text) {
      storage.setItem(key, text)
    },
    async remove(key) {
      storage.removeItem(key)
    },
  }
}

/** This window's cache, encrypted by the main process when it can. */
export function appMirrorCache(): MirrorCache {
  const bridge = typeof window === 'undefined' ? null : (window.codync?.app?.cache ?? null)
  if (bridge) {
    return new MirrorCache(bridge, typeof localStorage === 'undefined' ? null : localStorage)
  }
  const storage = typeof localStorage === 'undefined' ? null : localStorage
  return new MirrorCache(storage ? storageBackend(storage) : {
    read: async () => null,
    write: async () => {},
    remove: async () => {},
  })
}
