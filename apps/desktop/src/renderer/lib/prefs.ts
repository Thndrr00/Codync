import type { AppLanguage } from './l10n.ts'
import { resolvedLanguage } from './l10n.ts'
import { useSyncExternalStore } from 'react'
import type { StarAsk } from './star-ask'
import type { KofiInvite } from './kofi-invite'
import type { YourBubbles } from './theme'
import { NO_ROSTER_VIEW_FILTER, type RosterViewFilter } from './roster-view-filter.ts'
import { inkOn } from './bubble-colors.ts'
import type { SavedTabsByChat } from './open-tabs.ts'
import { DEFAULT_BINDINGS, sanitizeBindings, type BindingMap } from '../components/keyboard-shortcuts.ts'

/** A stored value, or the fallback when it's missing, unreadable or of another kind (a `null` where a string
 * belongs crashed the chat window). A `null` fallback takes any value. A validator can narrow a value
 * whose JSON type is right but whose contents are not (a keyboard binding map, say). */
function stored<T>(raw: string | null, fallback: T, validate?: (value: unknown, fallback: T) => T): T {
  if (raw === null) return fallback
  try {
    const value: unknown = JSON.parse(raw)
    if (validate) return validate(value, fallback)
    const sameKind = value !== null && typeof value === typeof fallback && Array.isArray(value) === Array.isArray(fallback)
    return fallback === null || sameKind ? (value as T) : fallback
  } catch {
    return fallback
  }
}

/** A boolean saved under an older preference key, so renaming a key doesn't ask its question again. */
function savedChoice(oldKey: string): boolean {
  try {
    return localStorage.getItem(oldKey) === 'true'
  } catch {
    return false
  }
}

/** A per-window preference in localStorage (kit's `@AppStorage`), observable by views. */
export function pref<T>(key: string, fallback: T, validate?: (value: unknown, fallback: T) => T) {
  const listeners = new Set<() => void>()
  let value: T = (() => {
    try {
      return stored(localStorage.getItem(key), fallback, validate)
    } catch {
      return fallback
    }
  })()
  window.addEventListener('storage', (e) => {
    if (e.key !== key) return
    value = stored(e.newValue, fallback, validate)
    for (const l of listeners) l()
  })
  return {
    get: () => value,
    set(next: T) {
      value = next
      try {
        localStorage.setItem(key, JSON.stringify(next))
      } catch {}
      for (const l of listeners) l()
    },
    subscribe(l: () => void) {
      listeners.add(l)
      return () => void listeners.delete(l)
    },
  }
}

export type Pref<T> = ReturnType<typeof pref<T>>

export function usePref<T>(p: Pref<T>): [T, (v: T) => void] {
  const value = useSyncExternalStore(p.subscribe, p.get)
  return [value, p.set]
}

/**
 * A font preference stores one family name, never a CSS declaration: only letters, digits, spaces and
 * a few typographic marks are kept, so a stored or pasted value cannot inject styles or URLs.
 */
const FONT_NAME = /^[\p{L}\p{N}][\p{L}\p{N} _+'.-]{0,79}$/u

export function validFontFamily(value: unknown): value is string {
  return typeof value === 'string' && FONT_NAME.test(value)
}

/** Quotes a family with spaces, then appends a trusted stack. `fallback` is always a constant, never user input. */
export function font_stack(candidate: string | null, fallback: string): string {
  if (!validFontFamily(candidate)) return fallback
  const family = /\s/.test(candidate) ? `"${candidate}"` : candidate
  return `${family}, ${fallback}`
}

export const TEXT_SIZES = [11, 12, 13, 14, 15, 16, 18]
export const DEFAULT_TEXT_SIZE = 12

/** The next larger (`up`) or smaller size for ⌘+ / ⌘−, staying at either end. */
export function stepTextSize(size: number, up: boolean) {
  return (up ? TEXT_SIZES.find((s) => s > size) : [...TEXT_SIZES].reverse().find((s) => s < size)) ?? size
}

export const prefs = {
  textSize: pref('conversationFontSize', DEFAULT_TEXT_SIZE),
  sidebarCompact: pref('sidebarCompact', false),
  sidebarWidth: pref('desktopSidebarWidth', 266),
  detailsPanelWidth: pref('detailsPanelWidth', 400),
  hiddenComputers: pref('hiddenComputers', ''),
  computerOrder: pref<string[]>('computerOrder', []),
  collapsedComputers: pref<string[]>('collapsedComputerSections', []),
  /** Rail pages hidden from its right-click menu; Credits is never one of them. */
  hiddenRailItems: pref<string[]>('hiddenRailItems', []),
  /** Folded roster sections, by name: on this device only. */
  rosterSectionOrder: pref<Record<string, string[]>>('rosterSectionOrder', {}),
  collapsedRosterSections: pref<string[]>('collapsedRosterSections', []),
  collapsedProjects: pref<string[]>('collapsedProjects', []),
  usageIconStyle: pref<'character' | 'original'>('usageIconStyle', 'character'),
  /** Bot characters are shaded (plush) or even dots (flat), as the phones draw them. */
  avatarStyle: pref<'plush' | 'flat'>('avatarStyle', 'plush'),
  /** Frosted layers keep their backdrop blur; off drops every one of them (a slower computer, sharper text). */
  backgroundBlur: pref('backgroundBlur', true),
  /** A transparent window or a chat backdrop: the Threads composer needs a fill behind it. */
  transparentWindow: pref('transparentWindow', false),
  /** Bot faces blink, look around and breathe; off keeps their still frame. */
  movingAgentFaces: pref('movingAgentFaces', true),
  /** The opening agent pile; off shows one agent with bouncing dots while a computer opens or connects. */
  startupAnimation: pref('startupAnimation', true),
  /** The roster's filter and sort, saved together on this device. */
  rosterViewFilter: pref<RosterViewFilter>('rosterViewFilter', NO_ROSTER_VIEW_FILTER),
  /** Your bubbles take the agent's avatar colour: Solid fills them, Tint mixes a little in, Classic keeps the chat's grey. */
  yourBubbles: pref<YourBubbles>('yourBubbles', 'solid'),
  /** The roster marks each bot with its agent's logo while the bots run more than one agent. */
  agentLogos: pref('showAgentLogos', true),
  onboardingDone: pref('macAccountOnboardingCompleted', false),
  /** Setup-checklist steps this device chose to dismiss; they stay hidden on later launches. */
  onboardingChecklistDismissed: pref<string[]>('onboardingChecklistDismissed', []),
  /** Chose to go on without signing in; signing in ends it. A choice saved under the old key (`accountWelcomeSkipped`) still counts. */
  withoutAccount: pref('withoutAccount', savedChoice('accountWelcomeSkipped')),
  computerAccessSetup: pref<'ready' | 'later' | null>('computerAccessSetup', null),
  starAsk: pref<StarAsk>('githubStarAsk', { kind: 'never' }),
  /** Prompts this computer delivered, voice calls aside; they bring the second star ask. */
  promptsSent: pref('promptsSent', 0),
  kofiInvite: pref<KofiInvite>('kofiInvite', { kind: 'never' }),
  appLanguage: pref<AppLanguage>('appLanguage', 'system'),
  /** The working pill unfolds the turn's thinking and steps; off keeps the chat to its messages. */
  thinkingDetails: pref('showThinkingDetails', false),
  /** Compact message actions (OpenBot): the per-message time under each bubble stays hidden. */
  compactActions: pref('compactMessageActions', false),
  extensionsMovedTipSeen: pref('extensionsMovedTipSeen', false),
  /** The version this device last opened on, for What's new; `null` before it ever recorded one. */
  whatsNewSeen: pref<string | null>('whatsNewSeen', null),
  /** A hand-picked accent for Settings › Appearance; `null` keeps the theme's own black/white accent. */
  accentColor: pref<string | null>('accentColor', null),
  /** Settings › Appearance: an installed family for app chrome, or `null` for the theme's default stack. */
  interfaceFont: pref<string | null>('interfaceFont', null, (value, fallback) => validFontFamily(value) ? value : fallback),
  /** Settings › Appearance: an installed family for code, or `null` for the theme's default stack. */
  codeFont: pref<string | null>('codeFont', null, (value, fallback) => validFontFamily(value) ? value : fallback),
  /** "Restore the last workspace on launch": brings back a chat's open file and artifact tabs. */
  restoreLastWorkspace: pref('restoreLastWorkspace', true),
  /** Open tabs of each chat, keyed by computer and bot; ids only, kept on this device. */
  openTabsByChat: pref<SavedTabsByChat>('openTabsByChat', {}),
  /** Open workflow tabs in the flow title bar, in open order; workflow (bot) ids only, on this device. */
  openWorkflowTabs: pref<string[]>('openWorkflowTabs', []),
  /** The command palette's last picks, newest first, leading its Recent group. */
  recentCommands: pref<string[]>('recentCommands', []),
  /** Global shortcut chords changed in Settings › Keyboard; every invalid or duplicate one falls back to its default. */
  keyboardBindings: pref<BindingMap>('keyboardBindings', DEFAULT_BINDINGS, sanitizeBindings),
}

/** The Mac preference supplies one scale for conversation text and app chrome. */
export function applyTextSize(size: number) {
  const clamped = Number.isFinite(size) ? Math.min(Math.max(size, 11), 18) : DEFAULT_TEXT_SIZE
  document.documentElement.style.setProperty('--scale', String(clamped / DEFAULT_TEXT_SIZE))
  document.documentElement.style.setProperty('--conversation', `${clamped}px`)
  // Match the reference chat at the selected size; keep larger text an explicit preference.
  document.documentElement.style.setProperty('--chat', `${clamped}px`)
  document.documentElement.style.setProperty('--chat-scale', String(clamped / DEFAULT_TEXT_SIZE))
}

prefs.textSize.subscribe(() => applyTextSize(prefs.textSize.get()))

/**
 * Background blur off marks the body, and one rule in `styles/theme.css` drops every frosted
 * surface's backdrop filter, so no CSS file keeps its own copy of the setting.
 */
export function applyBackgroundBlur(on: boolean) {
  if (typeof document === 'undefined' || !document.body) return
  if (on) delete document.body.dataset.blur
  else document.body.dataset.blur = 'off'
}

prefs.backgroundBlur.subscribe(() => applyBackgroundBlur(prefs.backgroundBlur.get()))
applyBackgroundBlur(prefs.backgroundBlur.get())

/**
 * A custom accent is written on the root as inline CSS variables, so it wins over both colour schemes
 * in `theme.css`. The ink on an accent fill is picked for contrast, so it reads in light and dark; `null`
 * removes the override and restores the curated accent. The curated swatches themselves are untouched.
 */
export function applyAccentColor(hex: string | null) {
  if (typeof document === 'undefined') return
  const style = document.documentElement.style
  if (!hex) {
    for (const name of ['--accent', '--accent-fill', '--on-accent']) style.removeProperty(name)
    return
  }
  style.setProperty('--accent', hex)
  style.setProperty('--accent-fill', hex)
  style.setProperty('--on-accent', inkOn(hex))
}

prefs.accentColor.subscribe(() => applyAccentColor(prefs.accentColor.get()))
applyAccentColor(prefs.accentColor.get())

/* The default stacks are the ones in styles/theme.css; keep both in step. A chosen family is prepended
   to its stack as an inline root variable, so every view picks it up without its own font CSS. */
const DEFAULT_INTERFACE_STACK = '-apple-system, BlinkMacSystemFont, "SF Pro Text", "Inter", "Segoe UI", system-ui, sans-serif'
const DEFAULT_CODE_STACK = 'ui-monospace, "SF Mono", SFMono-Regular, Menlo, "DejaVu Sans Mono", monospace'

export function applyFonts(interfaceFamily: string | null, codeFamily: string | null) {
  if (typeof document === 'undefined') return
  const style = document.documentElement.style
  if (validFontFamily(interfaceFamily)) style.setProperty('--font', font_stack(interfaceFamily, DEFAULT_INTERFACE_STACK))
  else style.removeProperty('--font')
  if (validFontFamily(codeFamily)) style.setProperty('--mono', font_stack(codeFamily, DEFAULT_CODE_STACK))
  else style.removeProperty('--mono')
}

prefs.interfaceFont.subscribe(() => applyFonts(prefs.interfaceFont.get(), prefs.codeFont.get()))
prefs.codeFont.subscribe(() => applyFonts(prefs.interfaceFont.get(), prefs.codeFont.get()))
applyFonts(prefs.interfaceFont.get(), prefs.codeFont.get())

/** The window mirrors for Arabic; paragraphs still follow their own text. */
export function applyDirection(language: AppLanguage) {
  if (typeof document === 'undefined') return
  document.documentElement.dir = resolvedLanguage(language, typeof navigator === 'undefined' ? 'en' : navigator.language) === 'ar' ? 'rtl' : 'ltr'
}

prefs.appLanguage.subscribe(() => applyDirection(prefs.appLanguage.get()))
applyDirection(prefs.appLanguage.get())
