import type { UsageProvider, UsageWindow } from '@shared/models'

/** Account report IDs are distinct; their icon still belongs to the agent. */
export function providerRegistry(id: string) {
  const backend = id.split(':')[0]
  return ['claude', 'codex', 'antigravity'].includes(backend) ? `${backend}-acp` : backend
}

export function windowTitle(window: UsageWindow) {
  if (window.label === '5-hour') return 'Session'
  return window.label.startsWith('Weekly · ') ? window.label.slice('Weekly · '.length) : window.label
}

export function usageStatus(provider: UsageProvider) {
  if (provider.status === 'unsupported') return 'Update this agent to read its plan limits.'
  return provider.status ? 'Plan limits are unavailable for this account.' : null
}
