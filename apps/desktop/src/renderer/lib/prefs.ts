import type { AppLanguage } from './l10n.ts'
import { readsRightToLeft } from './l10n.ts'
import { useSyncExternalStore } from 'react'
import type { StarAsk } from './star-ask'
import type { KofiInvite } from './kofi-invite'
import { mixHex, savedChatBackground, savedChatColour } from './chat-appearance'

/** A stored value, or the fallback when it's missing, unreadable or of another kind (a `null` where a string
 * belongs crashed the chat window). A `null` fallback takes any value. */
function stored<T>(raw: string | null, fallback: T): T {
  if (raw === null) return fallback
  try {
    const value: unknown = JSON.parse(raw)
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
export function pref<T>(key: string, fallback: T) {
  const listeners = new Set<() => void>()
  let value: T = (() => {
    try {
      return stored(localStorage.getItem(key), fallback)
    } catch {
      return fallback
    }
  })()
  window.addEventListener('storage', (e) => {
    if (e.key !== key) return
    value = stored(e.newValue, fallback)
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
  /** Folded roster sections, by name: on this device only. */
  rosterSectionOrder: pref<Record<string, string[]>>('rosterSectionOrder', {}),
  collapsedRosterSections: pref<string[]>('collapsedRosterSections', []),
  collapsedProjects: pref<string[]>('collapsedProjects', []),
  usageIconStyle: pref<'character' | 'original'>('usageIconStyle', 'character'),
  /** Bot characters are shaded (plush) or even dots (flat), as the phones draw them. */
  avatarStyle: pref<'plush' | 'flat'>('avatarStyle', 'plush'),
  /** The roster marks each bot with its agent's logo while the bots run more than one agent. */
  agentLogos: pref('showAgentLogos', true),
  onboardingDone: pref('macAccountOnboardingCompleted', false),
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
  extensionsMovedTipSeen: pref('extensionsMovedTipSeen', false),
  /** The version this device last opened on, for What's new; `null` before it ever recorded one. */
  whatsNewSeen: pref<string | null>('whatsNewSeen', null),
  /** The person's own bubble colour: "light/dark" preset ids, `-` for the theme's own. */
  chatColour: pref<string>('chatColour', ''),
  /** The colour behind the chat: "light/dark" preset ids, `-` for the theme's own. */
  chatBackground: pref<string>('chatBackground', ''),
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
 * The chat colour and background of the scheme the window shows, as CSS variables. The other
 * scheme keeps its own choice and appears when the system switches.
 */
export function applyChatAppearance() {
  if (typeof document === 'undefined' || typeof window === 'undefined' || typeof window.matchMedia !== 'function') return
  const dark = window.matchMedia('(prefers-color-scheme: dark)').matches
  const root = document.documentElement.style
  const colour = savedChatColour(prefs.chatColour.get(), dark)
  if (colour) {
    const base = dark ? '#3A3A3A' : '#E2E2E2'
    root.setProperty('--chat-bubble-user', mixHex(base, dark ? colour.dark : colour.light, dark ? 0.22 : 0.14))
  } else {
    root.removeProperty('--chat-bubble-user')
  }
  const background = savedChatBackground(prefs.chatBackground.get(), dark)
  if (background) root.setProperty('--chat-background', dark ? background.dark : background.light)
  else root.removeProperty('--chat-background')
}

prefs.chatColour.subscribe(applyChatAppearance)
prefs.chatBackground.subscribe(applyChatAppearance)
if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', applyChatAppearance)
}
applyChatAppearance()

/** Arabic and Hebrew read right to left; paragraphs still follow their own text. */
export function applyDirection(language: AppLanguage) {
  if (typeof document === 'undefined') return
  document.documentElement.dir = readsRightToLeft(language) ? 'rtl' : 'ltr'
}

prefs.appLanguage.subscribe(() => applyDirection(prefs.appLanguage.get()))
applyDirection(prefs.appLanguage.get())
