/**
 * The chat window's real keyboard shortcuts, in one list. The cheatsheet renders this list, the
 * command palette reads the same entries for its shortcut hints, Settings › Keyboard captures
 * bindings for the rows the window owns, and the global key handler matches the same map. One
 * effective binding map (`prefs.keyboardBindings`) drives all four, so they cannot drift apart.
 */

/** A platform-neutral chord. `mod` is ⌘ on macOS and Ctrl on Linux/Windows. */
export interface Chord {
  mod: boolean
  alt: boolean
  shift: boolean
  /** `event.key`, with letters lowercased and Space spelled out. */
  key: string
}

/** The effective binding of each rebindable shortcut id. */
export type BindingMap = Record<string, Chord>

/** The shape a real `KeyboardEvent` needs for matching and capture; keeps both helpers pure. */
export interface ChordEvent {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
  shiftKey: boolean
  isComposing?: boolean
  defaultPrevented?: boolean
}

export interface Shortcut {
  id: string
  group: string
  label: string
  /** The built-in keys, as shown for rows whose keys belong to a text field or a list. */
  keys: string
  /** The default binding of a row the window's key handler owns. */
  chord?: Chord
  /** False for keys owned by the composer, roster or another view: shown, but not rebound here. */
  rebindable?: boolean
}

/** The keys that carry no chord of their own; pressing one alone cannot be a binding. */
const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta', 'AltGraph', 'CapsLock'])

/** Keys the app needs to keep: closing overlays, moving focus and sending a message. */
export const RESERVED_CHORDS: Chord[] = [
  { mod: false, alt: false, shift: false, key: 'Escape' },
  { mod: false, alt: false, shift: false, key: 'Tab' },
  { mod: false, alt: false, shift: true, key: 'Tab' },
  { mod: false, alt: false, shift: false, key: 'Enter' },
  { mod: false, alt: false, shift: true, key: 'Enter' },
]

/** `event.key` as stored: lowercase letters and a named Space, everything else verbatim. */
export function normalizeKey(key: string): string {
  if (key === ' ') return 'Space'
  return /^[a-z]$/i.test(key) ? key.toLowerCase() : key
}

/** The identity of a chord, used for duplicate checks and stored comparisons. */
export function chordKey(chord: Chord): string {
  return `${chord.mod ? 'mod+' : ''}${chord.alt ? 'alt+' : ''}${chord.shift ? 'shift+' : ''}${chord.key}`
}

/** What a chord reads like on one platform; `mod` and `alt` follow the platform's symbols. */
export function chordLabel(chord: Chord, platform: string): string {
  const mac = platform === 'darwin'
  const key = chord.key === 'Space' ? 'Space' : chord.key.length === 1 ? chord.key.toUpperCase() : chord.key
  return `${chord.mod ? (mac ? '⌘' : 'Ctrl+') : ''}${chord.alt ? (mac ? '⌥' : 'Alt+') : ''}${chord.shift ? (mac ? '⇧' : 'Shift+') : ''}${key}`
}

/** Both platforms in one hint, for the cheatsheet's built-in rows. */
function chordLabelPair(chord: Chord): string {
  return `${chordLabel(chord, 'darwin')} / ${chordLabel(chord, 'win32')}`
}

const commandPalette: Chord = { mod: true, alt: false, shift: false, key: 'k' }
const keyboardShortcuts: Chord = { mod: true, alt: false, shift: false, key: '/' }
const findInChat: Chord = { mod: true, alt: false, shift: false, key: 'f' }
const toggleSidebar: Chord = { mod: true, alt: false, shift: false, key: 'b' }
const openSettings: Chord = { mod: true, alt: false, shift: false, key: ',' }
const openRosterSearch: Chord = { mod: true, alt: false, shift: true, key: 'f' }
const addAgent: Chord = { mod: true, alt: false, shift: true, key: 'e' }
const unreadPrevious: Chord = { mod: true, alt: false, shift: true, key: 'j' }
const unreadNext: Chord = { mod: true, alt: false, shift: false, key: 'j' }
const muteSelected: Chord = { mod: true, alt: false, shift: true, key: 'm' }

export const KEYBOARD_SHORTCUTS: Shortcut[] = [
  { id: 'commandPalette', group: 'General', label: 'Open command palette', keys: chordLabelPair(commandPalette), chord: commandPalette, rebindable: true },
  { id: 'keyboardShortcuts', group: 'General', label: 'Show keyboard shortcuts', keys: `? (${chordLabel(keyboardShortcuts, 'darwin')} / ${chordLabel(keyboardShortcuts, 'win32')})`, chord: keyboardShortcuts, rebindable: true },
  { id: 'findInChat', group: 'General', label: 'Search in chat', keys: chordLabelPair(findInChat), chord: findInChat, rebindable: true },
  { id: 'closeOverlay', group: 'General', label: 'Close a sheet or menu', keys: 'Esc' },
  { id: 'toggleSidebar', group: 'General', label: 'Toggle the sidebar', keys: chordLabelPair(toggleSidebar), chord: toggleSidebar, rebindable: true },
  { id: 'openSettings', group: 'General', label: 'Open settings', keys: chordLabelPair(openSettings), chord: openSettings, rebindable: true },
  { id: 'openRosterSearch', group: 'General', label: 'Search bots and history', keys: chordLabelPair(openRosterSearch), chord: openRosterSearch, rebindable: true },
  { id: 'addAgent', group: 'General', label: 'Add an agent', keys: chordLabelPair(addAgent), chord: addAgent, rebindable: true },
  { id: 'botPrevious', group: 'Bots', label: 'Select the previous bot', keys: '↑' },
  { id: 'botNext', group: 'Bots', label: 'Select the next bot', keys: '↓' },
  { id: 'botMove', group: 'Bots', label: 'Move the selected bot', keys: 'Alt+↑ / Alt+↓' },
  { id: 'botActions', group: 'Bots', label: 'Show a bot’s quick actions', keys: '→' },
  { id: 'botPreview', group: 'Bots', label: 'Preview a bot', keys: 'Space' },
  { id: 'botNumber', group: 'Bots', label: 'Open a numbered search result', keys: '⌘1–9 / Ctrl+1–9' },
  { id: 'unreadPrevious', group: 'Bots', label: 'Jump to the previous unread bot', keys: chordLabelPair(unreadPrevious), chord: unreadPrevious, rebindable: true },
  { id: 'unreadNext', group: 'Bots', label: 'Jump to the next unread bot', keys: chordLabelPair(unreadNext), chord: unreadNext, rebindable: true },
  { id: 'muteSelected', group: 'Notifications', label: 'Mute or unmute the selected bot', keys: chordLabelPair(muteSelected), chord: muteSelected, rebindable: true },
  { id: 'send', group: 'Conversation', label: 'Send a message', keys: 'Enter' },
  { id: 'newline', group: 'Conversation', label: 'Add a line', keys: 'Shift+Enter' },
  { id: 'saveRoutine', group: 'Editing', label: 'Save a routine', keys: '⌘S / Ctrl+S' },
  { id: 'findInFlow', group: 'Editing', label: 'Find in a flow', keys: '⌘G / Ctrl+G' },
]

/** Every row the Keyboard settings page can change, in cheatsheet order. */
export const REBINDABLE_SHORTCUTS = KEYBOARD_SHORTCUTS.filter((shortcut) => shortcut.rebindable && shortcut.chord)

/** The bindings a device starts with, and what Reset all restores. */
export const DEFAULT_BINDINGS: BindingMap = Object.fromEntries(REBINDABLE_SHORTCUTS.map((shortcut) => [shortcut.id, shortcut.chord!]))

export function isReservedChord(chord: Chord): boolean {
  return RESERVED_CHORDS.some((reserved) => chordKey(reserved) === chordKey(chord))
}

/** Only a complete, well-formed chord can be kept: no lone modifier or unknown shape. */
export function isValidChord(value: unknown): value is Chord {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const chord = value as Record<string, unknown>
  return typeof chord.mod === 'boolean' && typeof chord.alt === 'boolean' && typeof chord.shift === 'boolean' &&
    typeof chord.key === 'string' && chord.key.length > 0 && chord.key.length <= 24 && !MODIFIER_KEYS.has(chord.key)
}

/**
 * Reads a stored binding map. An override wins only when it is well-formed, not reserved and does
 * not take another row's chord (defaults are unique); everything else falls back to the default.
 */
export function resolveBindings(value: unknown): BindingMap {
  const overrides = value && typeof value === 'object' && !Array.isArray(value) ? value as BindingMap : {}
  const defaultsByChord = new Map(REBINDABLE_SHORTCUTS.map((shortcut) => [chordKey(shortcut.chord!), shortcut.id]))
  const accepted = new Map<string, Chord>()
  const taken = new Set<string>()
  for (const shortcut of REBINDABLE_SHORTCUTS) {
    const candidate = overrides[shortcut.id]
    if (!isValidChord(candidate) || isReservedChord(candidate)) continue
    const signature = chordKey(candidate)
    if (defaultsByChord.has(signature) && defaultsByChord.get(signature) !== shortcut.id) continue
    if (taken.has(signature)) continue
    accepted.set(shortcut.id, candidate)
    taken.add(signature)
  }
  return Object.fromEntries(REBINDABLE_SHORTCUTS.map((shortcut) => [shortcut.id, accepted.get(shortcut.id) ?? shortcut.chord!]))
}

/** The validator a device pref uses: any stored junk resolves back to a valid, complete map. */
export function sanitizeBindings(value: unknown): BindingMap {
  return resolveBindings(value)
}

export type CaptureResult = { ok: true; chord: Chord } | { ok: false; reason: 'cancelled' | 'modifier' | 'reserved' | 'duplicate' | 'empty' }

/**
 * Reads one key press while a row is recording. Escape cancels, a modifier on its own is not a
 * chord, and reserved or already-used chords are refused. `taken` excludes the row being changed.
 */
export function captureChord(event: ChordEvent, taken: Iterable<Chord>): CaptureResult {
  if (event.isComposing) return { ok: false, reason: 'empty' }
  if (event.key === 'Escape') return { ok: false, reason: 'cancelled' }
  const key = normalizeKey(event.key)
  if (!key || MODIFIER_KEYS.has(event.key)) return { ok: false, reason: 'modifier' }
  const chord: Chord = { mod: event.metaKey || event.ctrlKey, alt: event.altKey, shift: event.shiftKey, key }
  if (isReservedChord(chord)) return { ok: false, reason: 'reserved' }
  const signature = chordKey(chord)
  for (const other of taken) {
    if (chordKey(other) === signature) return { ok: false, reason: 'duplicate' }
  }
  return { ok: true, chord }
}

/** Whether a key event fires a chord. `?` is its own character, so Shift is implied by it. */
export function matchesChord(event: ChordEvent, chord: Chord): boolean {
  if (event.isComposing || event.defaultPrevented || normalizeKey(event.key) !== chord.key) return false
  if ((event.metaKey || event.ctrlKey) !== chord.mod || event.altKey !== chord.alt) return false
  if (chord.key === '?') return true
  return event.shiftKey === chord.shift
}

/**
 * The id of the global shortcut an event fires, or none. While typing, only the command palette
 * is allowed through: the other keys belong to the text field and to IME composition.
 */
export function shortcutForEvent(event: ChordEvent, bindings: BindingMap, typing = false): string | undefined {
  const id = KEYBOARD_SHORTCUTS.find((shortcut) => {
    const chord = bindings[shortcut.id]
    return chord ? matchesChord(event, chord) : false
  })?.id
  if (!id || (typing && id !== 'commandPalette')) return undefined
  return id
}

/** The keys shown for a shortcut id: its effective binding for a rebindable row, its fixed keys otherwise. */
export function shortcutKeys(id: string, bindings?: BindingMap, platform = 'darwin'): string | undefined {
  const shortcut = KEYBOARD_SHORTCUTS.find((entry) => entry.id === id)
  if (!shortcut) return undefined
  const chord = shortcut.rebindable ? bindings?.[id] ?? shortcut.chord : undefined
  if (!chord) return shortcut.keys
  const label = chordLabel(chord, platform)
  // `?` still opens the sheet after ⌘/ is rebound; keep the hint while ⌘/ is the default.
  return id === 'keyboardShortcuts' && chordKey(chord) === chordKey(keyboardShortcuts) ? `${label} (?)` : label
}
