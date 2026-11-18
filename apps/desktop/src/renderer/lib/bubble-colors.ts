/**
 * The colour maths behind avatar-coloured bubbles (OpenBot 0.72.0's "Your bubbles"): a fill, the ink
 * that reads on it, and a fill nudged until its ink passes WCAG 4.5:1. Kept apart from `theme.ts` so
 * the rules can be tested without a window.
 */

/** `[r, g, b]` → `#RRGGBB`. */
export const rgbHex = (rgb: number[]) => `#${rgb.map((c) => c.toString(16).padStart(2, '0')).join('')}`

/** The avatar colours a Solid bubble can take, moved here so the contrast can be tested without a window; `theme.ts` re-exports them. */
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

/** `#RRGGBB` → `rgba(r, g, b, a)`. */
export function withAlpha(hex: string, alpha: number) {
  const n = parseInt(hex.slice(1), 16)
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`
}
