/**
 * Export/import of this device's UI preferences. Pure on purpose: it takes a plain snapshot of values
 * and returns JSON, and parses JSON back into validated groups. No `localStorage`, no `window`, no host,
 * so `node --test` can check the round-trip, a foreign version and that no secret field travels.
 *
 * Only the groups below are ever written or read. An unknown key in a caller's snapshot or in a file is
 * dropped, so a key, token, credential, account or host-side value cannot be exported by mistake.
 */

export const SETTINGS_IO_VERSION = 1
export const SETTINGS_IO_KIND = 'codync-settings'

export type SettingsGroup = 'theme' | 'roster' | 'bubbles' | 'motion' | 'voice' | 'appearance'

export interface SettingsGroupInfo {
  id: SettingsGroup
  label: string
}

/** The groups a person can pick for export, in the order they appear in settings. */
export const SETTINGS_GROUPS: readonly SettingsGroupInfo[] = [
  { id: 'theme', label: 'Theme (light, dark, system)' },
  { id: 'roster', label: 'Roster filter and sort' },
  { id: 'bubbles', label: 'Bubble style' },
  { id: 'motion', label: 'Motion' },
  { id: 'voice', label: 'Voice mode and talk key' },
  { id: 'appearance', label: 'Accent color' },
]

/** The only fields kept for each group. Anything else is a secret or someone else's data. */
const FIELDS: Record<SettingsGroup, readonly string[]> = {
  theme: ['appearance'],
  roster: ['rosterViewFilter'],
  bubbles: ['yourBubbles'],
  motion: ['movingAgentFaces', 'backgroundBlur'],
  voice: ['inputMode', 'talkKey'],
  appearance: ['accentColor'],
}

export type Values = Record<string, unknown>
export type SettingsSource = Partial<Record<SettingsGroup, Values>>

export interface ExportedSettings {
  app: string
  version: number
  exportedAt: string
  groups: Partial<Record<SettingsGroup, Values>>
}

export type ParseResult =
  | { ok: true; exportedAt: string | null; groups: Partial<Record<SettingsGroup, Values>>; skipped: { id: string; reason: string }[] }
  | { ok: false; error: string }

const plain = (v: unknown): v is Values => typeof v === 'object' && v !== null && !Array.isArray(v)
const oneOf = (v: unknown, choices: readonly string[]) => typeof v === 'string' && choices.includes(v)

/** `#RGB` or `#RRGGBB`, with or without the `#`, normalised to upper case. `null` for anything else. */
export function normalizeHex(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const match = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value.trim())
  if (!match) return null
  const digits = match[1]!
  const full = digits.length === 3 ? [...digits].map((c) => c + c).join('') : digits
  return `#${full.toUpperCase()}`
}

function isRosterFilter(value: unknown): boolean {
  if (!plain(value)) return false
  const allowed = ['show', 'unread', 'working', 'attention', 'pinned', 'sections', 'sort']
  return Object.entries(value).every(([key, item]) => {
    switch (key) {
      case 'show': return oneOf(item, ['all', 'agents', 'channels'])
      case 'sort': return oneOf(item, ['manual', 'recent'])
      case 'unread': case 'working': case 'attention': case 'pinned': return typeof item === 'boolean'
      case 'sections': return Array.isArray(item) && item.every((s) => typeof s === 'string')
      default: return !allowed.includes(key)
    }
  })
}

function isTalkKey(value: unknown): boolean {
  if (oneOf(value, ['ctrlOption', 'ctrlWindows'])) return true
  return plain(value) && ['ctrl', 'alt', 'meta', 'shift'].every((key) => typeof value[key] === 'boolean')
}

/** Keeps a group's known fields and checks their shape; `null` means the group is unusable. */
function validate(id: SettingsGroup, raw: unknown): Values | null {
  if (!plain(raw) || !(id in FIELDS)) return null
  const value: Values = {}
  for (const key of FIELDS[id]) if (key in raw) value[key] = raw[key]
  switch (id) {
    case 'theme': return oneOf(value.appearance, ['system', 'light', 'dark']) ? value : null
    case 'roster': return isRosterFilter(value.rosterViewFilter) ? value : null
    case 'bubbles': return oneOf(value.yourBubbles, ['solid', 'tint', 'classic']) ? value : null
    case 'motion':
      return Object.keys(value).length > 0 && ['movingAgentFaces', 'backgroundBlur'].every((k) => !(k in value) || typeof value[k] === 'boolean') ? value : null
    case 'voice':
      return Object.keys(value).length > 0 && (!('inputMode' in value) || oneOf(value.inputMode, ['pushToTalk', 'openMic']))
        && (!('talkKey' in value) || isTalkKey(value.talkKey)) ? value : null
    case 'appearance': {
      if (!('accentColor' in value)) return null
      if (value.accentColor === null) return value
      const hex = normalizeHex(value.accentColor)
      return hex ? { accentColor: hex } : null
    }
  }
}

/** Serialises the chosen groups of a device-local snapshot. `now` is injectable for a stable test. */
export function exportSettings(source: SettingsSource, include: readonly SettingsGroup[], now = new Date()): string {
  const groups: Partial<Record<SettingsGroup, Values>> = {}
  for (const id of include) {
    const clean = validate(id, source[id])
    if (clean) groups[id] = clean
  }
  const file: ExportedSettings = { app: SETTINGS_IO_KIND, version: SETTINGS_IO_VERSION, exportedAt: now.toISOString(), groups }
  return JSON.stringify(file, null, 2)
}

/** Reads a settings file: a clear error for a foreign version, and each group validated on its own. */
export function parseSettings(json: string): ParseResult {
  let raw: unknown
  try {
    raw = JSON.parse(json)
  } catch {
    return { ok: false, error: 'That file is not valid JSON.' }
  }
  if (!plain(raw) || raw.app !== SETTINGS_IO_KIND) return { ok: false, error: 'That is not a Codync settings file.' }
  if (raw.version !== SETTINGS_IO_VERSION) {
    return { ok: false, error: `That file uses settings format ${String(raw.version)}. This app reads format ${SETTINGS_IO_VERSION}.` }
  }
  const source = plain(raw.groups) ? raw.groups : {}
  const groups: Partial<Record<SettingsGroup, Values>> = {}
  const skipped: { id: string; reason: string }[] = []
  for (const { id, label } of SETTINGS_GROUPS) {
    if (!(id in source)) continue
    const clean = validate(id, source[id])
    if (clean) groups[id] = clean
    else skipped.push({ id, reason: `${label}: the values are not recognised.` })
  }
  const exportedAt = typeof raw.exportedAt === 'string' ? raw.exportedAt : null
  return { ok: true, exportedAt, groups, skipped }
}
