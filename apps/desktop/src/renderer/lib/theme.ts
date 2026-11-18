import { useSyncExternalStore } from 'react'

/**
 * A media query read from one MediaQueryList and listened to on another: Chromium shares a list's
 * `matches` with its change detection, so a read while the window is hidden can swallow the `change`
 * and memoized canvases (avatars) keep the old appearance's ink. Showing or focusing the window
 * checks again, so a missed change never outlives the next look at the window.
 */
function mediaQuery(media: string) {
  const reads = window.matchMedia(media)
  const events = window.matchMedia(media)
  return {
    get: () => reads.matches,
    subscribe: (cb: () => void) => {
      events.addEventListener('change', cb)
      document.addEventListener('visibilitychange', cb)
      window.addEventListener('focus', cb)
      return () => {
        events.removeEventListener('change', cb)
        document.removeEventListener('visibilitychange', cb)
        window.removeEventListener('focus', cb)
      }
    },
  }
}

const darkQuery = mediaQuery('(prefers-color-scheme: dark)')
const motionQuery = mediaQuery('(prefers-reduced-motion: reduce)')

/**
 * The screen's pixel ratio, for canvases' backing size. It changes when the window moves to another
 * screen or the Mac sleeps with the lid closed; a canvas drawn meanwhile would stay blurry at 1x.
 * The query names the current ratio, so it is renewed on every change.
 */
const pixelRatio = {
  get: () => window.devicePixelRatio || 1,
  subscribe: (cb: () => void) => {
    let query: MediaQueryList | null = null
    const changed = () => {
      watch()
      cb()
    }
    const watch = () => {
      query?.removeEventListener('change', changed)
      query = window.matchMedia(`(resolution: ${pixelRatio.get()}dppx)`)
      query.addEventListener('change', changed)
    }
    watch()
    document.addEventListener('visibilitychange', changed)
    window.addEventListener('focus', changed)
    return () => {
      query?.removeEventListener('change', changed)
      document.removeEventListener('visibilitychange', changed)
      window.removeEventListener('focus', changed)
    }
  },
}

export function useDark() {
  return useSyncExternalStore(darkQuery.subscribe, darkQuery.get)
}

export function useReduceMotion() {
  return useSyncExternalStore(motionQuery.subscribe, motionQuery.get)
}

export function usePixelRatio() {
  return useSyncExternalStore(pixelRatio.subscribe, pixelRatio.get)
}

export const isDark = darkQuery.get
export const reduceMotion = motionQuery.get

/** Palette values for drawing code (canvas) that can't read CSS variables; they match theme.css. */
export const palette = (dark: boolean) => ({
  text: dark ? [250, 250, 250] : [20, 20, 20],
  warning: '#F0A030',
  danger: dark ? '#F0A7A7' : '#C23A2B',
  added: dark ? '#8fd18b' : '#2e7d32',
  tertiary: dark ? '#9a9a9a' : '#6e6e6e',
})

export interface Swatch {
  id: string
  label: string
  hex: string
}

export const AVATAR_COLORS: Swatch[] = [
  { id: 'black', label: 'Black', hex: '#2B2B2B' },
  { id: 'brown', label: 'Brown', hex: '#936439' },
  { id: 'red', label: 'Red', hex: '#FF263C' },
  { id: 'orange', label: 'Orange', hex: '#FF6700' },
  { id: 'yellow', label: 'Yellow', hex: '#FF9800' },
  { id: 'green', label: 'Green', hex: '#00C972' },
  { id: 'cyan', label: 'Cyan', hex: '#00BCA6' },
  { id: 'blue', label: 'Blue', hex: '#1084FE' },
  { id: 'violet', label: 'Violet', hex: '#9159FE' },
  { id: 'magenta', label: 'Magenta', hex: '#FF309B' },
  { id: 'gray', label: 'Gray', hex: '#777777' },
]

export const AVATAR_SHAPES = ['blob', 'pebble', 'squircle', 'tablet', 'wedge', 'hex', 'cloud', 'teardrop']

export function avatarColor(id: string) {
  return (AVATAR_COLORS.find((c) => c.id === id) ?? AVATAR_COLORS[7]!).hex
}

/** How the person's own bubbles take an agent's avatar colour (`Settings › Appearance › Your bubbles`). */
export type YourBubbles = 'solid' | 'tint' | 'classic'

export const YOUR_BUBBLE_STYLES: { id: YourBubbles; label: string }[] = [
  { id: 'solid', label: 'Solid' },
  { id: 'tint', label: 'Tint' },
  { id: 'classic', label: 'Classic' },
]

/** WCAG relative luminance of an `#RRGGBB` colour (0 black … 1 white). */
function luminance(hex: string) {
  const n = parseInt(hex.slice(1), 16)
  const channel = (value: number) => {
    const c = value / 255
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel((n >> 16) & 255) + 0.7152 * channel((n >> 8) & 255) + 0.0722 * channel(n & 255)
}

/** WCAG contrast between two `#RRGGBB` colours (1 … 21). */
export function contrastRatio(first: string, second: string) {
  const a = luminance(first)
  const b = luminance(second)
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}

/** The ink that reads best on `fill`: the theme's near-black, or white. */
export function inkOn(fill: string) {
  return contrastRatio('#ffffff', fill) >= contrastRatio('#141414', fill) ? '#ffffff' : '#141414'
}

/** `fill` nudged away from its ink until that ink reads at 4.5:1, so a Solid bubble stays legible on every avatar colour. */
export function readableFill(fill: string) {
  const ink = inkOn(fill)
  // Away from the ink (toward white for dark ink, black for white ink): the fill keeps its hue and grows readable.
  const away = ink === '#141414' ? '#ffffff' : '#000000'
  const n = parseInt(fill.slice(1), 16)
  const a = parseInt(away.slice(1), 16)
  for (let step = 0; step <= 20; step++) {
    const t = step / 20
    const channel = (shift: number) => Math.round(((n >> shift) & 255) * (1 - t) + ((a >> shift) & 255) * t)
    const mixed = rgbHex([channel(16), channel(8), channel(0)])
    if (contrastRatio(ink, mixed) >= 4.5) return mixed
  }
  return away
}

/** `[r, g, b]` → `#RRGGBB`. */
export const rgbHex = (rgb: number[]) => `#${rgb.map((c) => c.toString(16).padStart(2, '0')).join('')}`

/** `#RRGGBB` → `rgba(r, g, b, a)`. */
export function withAlpha(hex: string, alpha: number) {
  const n = parseInt(hex.slice(1), 16)
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`
}
