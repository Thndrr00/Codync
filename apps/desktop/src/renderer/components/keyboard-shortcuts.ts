/**
 * The chat window's real keyboard shortcuts, in one list. The cheatsheet renders this list and the
 * command palette reads the same entries for its shortcut hints, so the two cannot drift apart.
 * `id` is the stable key views use to point at a row; `keys` is displayed as written (⌘ on a Mac).
 */
export interface Shortcut {
  id: string
  group: string
  label: string
  keys: string
}

export const KEYBOARD_SHORTCUTS: Shortcut[] = [
  { id: 'commandPalette', group: 'General', label: 'Open command palette', keys: '⌘K / Ctrl+K' },
  { id: 'keyboardShortcuts', group: 'General', label: 'Show keyboard shortcuts', keys: '?' },
  { id: 'closeOverlay', group: 'General', label: 'Close a sheet or menu', keys: 'Esc' },
  { id: 'botPrevious', group: 'Bots', label: 'Select the previous bot', keys: '↑' },
  { id: 'botNext', group: 'Bots', label: 'Select the next bot', keys: '↓' },
  { id: 'botMove', group: 'Bots', label: 'Move the selected bot', keys: 'Alt+↑ / Alt+↓' },
  { id: 'botActions', group: 'Bots', label: 'Show a bot’s quick actions', keys: '→' },
  { id: 'botPreview', group: 'Bots', label: 'Preview a bot', keys: 'Space' },
  { id: 'botNumber', group: 'Bots', label: 'Open a numbered search result', keys: '⌘1–9 / Ctrl+1–9' },
  { id: 'send', group: 'Conversation', label: 'Send a message', keys: 'Enter' },
  { id: 'newline', group: 'Conversation', label: 'Add a line', keys: 'Shift+Enter' },
  { id: 'saveRoutine', group: 'Editing', label: 'Save a routine', keys: '⌘S / Ctrl+S' },
  { id: 'findInFlow', group: 'Editing', label: 'Find in a flow', keys: '⌘G / Ctrl+G' },
]

/** The keys shown for a shortcut id, for a command's hint. */
export function shortcutKeys(id: string): string | undefined {
  return KEYBOARD_SHORTCUTS.find((shortcut) => shortcut.id === id)?.keys
}
