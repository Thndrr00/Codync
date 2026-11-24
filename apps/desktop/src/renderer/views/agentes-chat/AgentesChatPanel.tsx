//! The agentes.chat behaviors, in one panel: setup, presence, stretches, choice cards and the vault.
//! Views hold no I/O: every call goes through the typed bridge to the host, reached via the bot store.

import { useCallback, useEffect, useState } from 'react'
import { agentesChat, type ApprovalRequest, type PresenceStatus, type SetupChecklist, type StretchCard } from '../../client/agentes-chat'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import './agentes-chat.css'

export function AgentesChatPanel({ botId, onClose }: { botId: string; onClose: () => void }) {
  const store = useStore()
  const [setup, setSetup] = useState<SetupChecklist | null>(null)
  const [presence, setPresence] = useState<PresenceStatus | null>(null)
  const [status, setStatus] = useState('')
  const [card, setCard] = useState<StretchCard | null>(null)
  const [approval, setApproval] = useState<ApprovalRequest | null>(null)
  const [masked, setMasked] = useState('')
  const [secret, setSecret] = useState('')
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    try {
      const api = agentesChat(await store.ready())
      const [s, p] = await Promise.all([api.setup(botId), api.presence(botId)])
      setSetup(s)
      setPresence(p.status)
    } catch (e) {
      setError(String(e))
    }
  }, [store, botId])

  useEffect(() => { void load() }, [load])

  const saveStatus = async () => {
    try {
      const api = agentesChat(await store.ready())
      setPresence((await api.setPresence(botId, status)).status)
      setStatus('')
    } catch (e) { setError(String(e)) }
  }

  const makeCard = async () => {
    try {
      const api = agentesChat(await store.ready())
      setCard(await api.stretchCard('the bot', 'runs'))
      setApproval(await api.approvalCard('the bot', 'needs to send an email', 'request-1'))
    } catch (e) { setError(String(e)) }
  }

  const decide = async (option: string) => {
    if (!approval) return
    try {
      const api = agentesChat(await store.ready())
      setApproval(await api.decideApproval(approval, option))
    } catch (e) { setError(String(e)) }
  }

  const mask = async () => {
    try {
      const api = agentesChat(await store.ready())
      setMasked((await api.vaultMask('openai', secret)).masked)
    } catch (e) { setError(String(e)) }
  }

  return (
    <VStack className="agentes-chat-panel" gap={16}>
      <HStack className="agentes-chat-header" gap={8}>
        <Icon name="sparkles" size={18} />
        <Text weight="semibold">Agentes.chat</Text>
        <button className="agentes-chat-close" onClick={onClose} aria-label="Close">×</button>
      </HStack>

      {error && <Text tone="danger">{error}</Text>}

      <section className="agentes-chat-section">
        <Text weight="semibold">Setup</Text>
        {setup ? (
          <VStack gap={4}>
            {setup.steps.map((step) => (
              <HStack key={step.id} gap={8}>
                <Icon name={step.done ? 'checkmark.circle.fill' : 'circle'} size={14} />
                <Text>{step.label}</Text>
              </HStack>
            ))}
            <Text tone="secondary">{setup.done}/{setup.total} done</Text>
          </VStack>
        ) : <Text tone="secondary">Loading…</Text>}
      </section>

      <section className="agentes-chat-section">
        <Text weight="semibold">Presence</Text>
        <Text tone="secondary">{presence?.text ?? 'No status set'}</Text>
        <HStack gap={8}>
          <input value={status} onChange={(e) => setStatus(e.target.value)} placeholder="Back at 3" />
          <button onClick={() => void saveStatus()}>Set</button>
        </HStack>
      </section>

      <section className="agentes-chat-section">
        <button onClick={() => void makeCard()}>Preview cards</button>
        {card && <Text tone="secondary">{card.type}: {card.bodyCode}</Text>}
        {approval && (
          <VStack gap={4}>
            <Text>{approval.reason}</Text>
            {approval.status === 'pending' ? (
              <HStack gap={8}>
                {approval.options.map((o) => (
                  <button key={o.id} onClick={() => void decide(o.id)}>{o.label}</button>
                ))}
              </HStack>
            ) : <Text tone="secondary">Answered: {approval.selected}</Text>}
          </VStack>
        )}
      </section>

      <section className="agentes-chat-section">
        <Text weight="semibold">Vault</Text>
        <HStack gap={8}>
          <input type="password" value={secret} onChange={(e) => setSecret(e.target.value)} placeholder="secret" />
          <button onClick={() => void mask()}>Mask</button>
        </HStack>
        {masked && <Text tone="secondary">{masked}</Text>}
      </section>
    </VStack>
  )
}
