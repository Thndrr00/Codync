import type { IslandAction } from './island-activity'

/** The island can invoke only these existing intents, never an arbitrary host method. */
export function validIslandAction(input: unknown): IslandAction | null {
  if (!input || typeof input !== 'object') return null
  const action = input as Record<string, unknown>
  switch (action.kind) {
    case 'refreshUsage': case 'refreshActivity': case 'openChannels': case 'openDiagnostics':
      return { kind: action.kind }
    case 'callMute': case 'callEnd':
      return typeof action.callId === 'string' && action.callId.length > 0 && action.callId.length <= 128 ? { kind: action.kind, callId: action.callId } : null
    case 'stopBot': case 'openFlow':
      return typeof action.ref === 'string' && action.ref.length <= 512 && /^[^/]+\/.+$/.test(action.ref) ? { kind: action.kind, ref: action.ref } : null
    default:
      return null
  }
}
