import type { UsageWindow } from '@shared/models'
import { isDark } from '../../lib/theme'
export {
  isLimit,
  parseReset,
  providerRegistry,
  resetDate,
  resetDescription,
  tightest,
  untilText,
  usageStatus,
  windowTitle,
} from '../../lib/usage-presentation'

/** "62% used": the share of the cap already spent, said so (a bare percent reads as what's left). */
export const usedText = (percent: number, tr: (text: string) => string) =>
  tr('{percent}% used').replace('{percent}', String(Math.round(percent)))

/** The used percent, or the note when this window isn't a share of a cap. */
export function windowReading(w: UsageWindow, tr: (text: string) => string) {
  return w.text || usedText(w.percent, tr)
}

/** Claude keeps its orange; everyone else is ink, like the rest of the app. */
export function providerTint(id: string) {
  return id === 'claude' ? '#D97757' : isDark() ? '#FFFFFF' : '#000000'
}

/** CSS color for views (follows the scheme by itself). */
export const providerTintCSS = (id: string) => (id === 'claude' ? '#D97757' : 'var(--accent)')

/** The character that stands for this provider. */
export const mascotShape = (id: string) => (id === 'codex' ? 'hex' : 'blob')
