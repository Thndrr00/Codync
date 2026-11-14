// The desktop's mirror cache on disk: each computer's bots and recent messages, encrypted with the
// OS's own key (Keychain, DPAPI, libsecret) and written atomically. It is what makes the app open
// with content and read offline; the renderer never touches the file itself.

import { app, ipcMain, safeStorage } from 'electron'
import { mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { cacheFileName } from '../shared/cache-key'

/** Marks a cache written where the OS offers no key (a plain UTF-8 body follows). */
const PLAIN = 0x50

function directory() {
  return join(app.getPath('userData'), 'mirror-cache')
}

function file(key: string) {
  return join(directory(), cacheFileName(key))
}

function validKey(key: unknown): key is string {
  return typeof key === 'string' && key.length > 0 && key.length <= 200
}

export async function readMirrorCache(key: string): Promise<string | null> {
  if (!validKey(key)) return null
  try {
    const raw = await readFile(file(key))
    if (raw.length === 0) return null
    if (raw[0] === PLAIN) return raw.subarray(1).toString('utf8')
    if (!safeStorage.isEncryptionAvailable()) return null
    return safeStorage.decryptString(raw)
  } catch {
    // No cache yet, or one this OS key can no longer open: the host catches up from scratch.
    return null
  }
}

export async function writeMirrorCache(key: string, text: string): Promise<void> {
  if (!validKey(key) || typeof text !== 'string') return
  const body = safeStorage.isEncryptionAvailable()
    ? safeStorage.encryptString(text)
    : Buffer.concat([Buffer.from([PLAIN]), Buffer.from(text, 'utf8')])
  await mkdir(directory(), { recursive: true })
  const target = file(key)
  const temporary = `${target}.${process.pid}.tmp`
  await writeFile(temporary, body)
  await rename(temporary, target)
}

export async function clearMirrorCache(): Promise<void> {
  await rm(directory(), { recursive: true, force: true })
}

export function registerMirrorCache() {
  ipcMain.handle('cache:read', (_e, key: string) => readMirrorCache(key))
  ipcMain.handle('cache:write', (_e, key: string, text: string) => writeMirrorCache(key, text))
  ipcMain.handle('cache:clear', () => clearMirrorCache())
}
