import type { IslandAction } from './island-activity'
import { safeIslandGitHubUrl } from './island-integrations.ts'

/** The island can invoke only these existing intents, never an arbitrary host method. */
export function validIslandAction(input: unknown): IslandAction | null {
  if (!input || typeof input !== 'object') return null
  const action = input as Record<string, unknown>
  const ref = typeof action.ref === 'string' && action.ref.length <= 512 && /^[^/]+\/.+$/.test(action.ref) ? action.ref : null
  const id = typeof action.id === 'string' && action.id.length > 0 && action.id.length <= 256 ? action.id : null
  switch (action.kind) {
    case 'stopDictation': return id ? { kind: action.kind, id } : null
    case 'openGitHubLink': return ref && safeIslandGitHubUrl(action.url) ? { kind: action.kind, ref, url: action.url } : null
    case 'appendScratchpad':
      return ref && typeof action.text === 'string' && !!action.text.trim() && Array.from(action.text).length <= 4000 && typeof action.requestId === 'string' && action.requestId.length > 0 && action.requestId.length <= 128 ? { kind: action.kind, ref, text: action.text, requestId: action.requestId } : null
    case 'sendSavedDraft':
      return ref && typeof action.text === 'string' && !!action.text.trim() && action.text.length <= 1_000_000 && typeof action.original === 'string' && action.original.length <= 1_000_000 ? { kind: action.kind, ref, text: action.text, original: action.original } : null
    case 'refreshUsage': case 'refreshActivity': case 'openChannels': case 'openDiagnostics':
      return { kind: action.kind }
    case 'callMute': case 'callEnd':
      return typeof action.callId === 'string' && action.callId.length > 0 && action.callId.length <= 128 ? { kind: action.kind, callId: action.callId } : null
    case 'stopBot': case 'openFlow': case 'loadWorkspace': case 'resumeDraft': case 'loadIntegrations': case 'reconnectComputer': case 'openMemory':
      return ref ? { kind: action.kind, ref } : null
    case 'retryRun': case 'cancelQueued': case 'openArtifact': case 'openAgentTerminal': case 'openConnector': case 'openIntegrationArtifact': case 'runSavedSearch': case 'openBookmark': case 'openSearchHit':
      return ref && id ? { kind: action.kind, ref, id } : null
    case 'openFile':
      return ref && id && typeof action.entryId === 'string' && action.entryId.length > 0 && action.entryId.length <= 256 ? { kind: action.kind, ref, id, entryId: action.entryId } : null
    default:
      return null
  }
}
