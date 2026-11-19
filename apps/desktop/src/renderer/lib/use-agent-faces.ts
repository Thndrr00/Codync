import { prefs, usePref } from './prefs.ts'

/**
 * Whether bot faces may move (Settings › Appearance › Moving agent faces). Avatar components read
 * it when they get no `animated` of their own, so every face follows the device's choice.
 */
export function useMovingAgentFaces(): boolean {
  return usePref(prefs.movingAgentFaces)[0]
}
