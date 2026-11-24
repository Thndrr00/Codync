import { isAbsolute, parse, resolve, sep } from 'node:path'

/**
 * The only path the bot-workspace channel may open: the bot's own workspace folder, already absolute
 * on the host. `resolve` must leave it unchanged, so `..` cannot name a different folder.
 */
export function revealableFolder(cwd: unknown): string | null {
  if (typeof cwd !== 'string') return null
  const trimmed = cwd.trim()
  if (!trimmed || /[\0\r\n]/.test(trimmed) || !isAbsolute(trimmed)) return null
  const candidate = sep === '\\' ? trimmed.replace(/\//g, '\\') : trimmed
  const path = resolve(candidate)
  if (path === parse(path).root) return null
  const clean = (value: string) => value.replace(/[\\/]+$/, '')
  return clean(path) === clean(candidate) ? path : null
}
