//! Behaviors brought from agentes.chat: the host owns them, the app just calls and renders.
//! Kept in one small file so the client never reimplements host logic.

import type { HostClient } from './host-client'

export interface PresenceStatus {
  text: string
  until: number | null
}

export interface SetupStep {
  id: string
  done: boolean
  label: string
}

export interface SetupChecklist {
  steps: SetupStep[]
  done: number
  total: number
  complete: boolean
}

export interface StretchCard {
  type: 'stretch_paused'
  agent: string
  reason: string
  action: 'continuar'
  bodyCode: string
}

export interface ApprovalRequest {
  type: 'approval_request'
  requestId: string
  agent: string
  reason: string
  options: { id: string; label: string; kind: string }[]
  status: 'pending' | 'decided'
  selected?: string
}

export interface VaultEntry {
  name: string
  source: string
  masked: string
}

/** Typed calls to the five host methods; every list is what the host answered, never a local copy. */
export function agentesChat(host: HostClient) {
  return {
    async presence(botId: string) {
      return host.call<{ status: PresenceStatus | null }>('botPresence', { botId })
    },
    setPresence(botId: string, status: string, until?: number) {
      return host.call<{ status: PresenceStatus | null }>('botPresence', { botId, status, until })
    },
    async setup(botId: string) {
      return host.call<SetupChecklist>('botSetup', { botId })
    },
    async stretchCard(agent: string, reason: string) {
      return host.call<StretchCard>('stretchCard', { agent, reason })
    },
    async approvalCard(agent: string, reason: string, requestId: string) {
      return host.call<ApprovalRequest>('agentApproval', { agent, reason, requestId })
    },
    async decideApproval(card: ApprovalRequest, option: string) {
      return host.call<ApprovalRequest>('agentApproval', { op: 'decide', card, option })
    },
    async vaultMask(name: string, secret: string, source = 'keychain') {
      return host.call<VaultEntry>('vaultMask', { name, secret, source })
    },
  }
}
