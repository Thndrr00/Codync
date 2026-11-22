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

export { AVATAR_SHAPES, isAvatarShape, safeAvatarShape, type AvatarShape } from './avatar-shapes.ts'

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

import { AVATAR_COLORS } from './bubble-colors.ts'

export { AVATAR_COLORS, contrastRatio, inkOn, readableFill, rgbHex, withAlpha, type Swatch } from './bubble-colors.ts'
