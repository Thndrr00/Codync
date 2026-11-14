import { useState, type ReactNode } from 'react'
import { Button, ChoicePicker } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { signalChatOk, signalNumberOk, signalPersonOk, signalStateText, type SignalStatus } from '../../../shared/channels'
import { useTr } from '../../lib/use-language'
import { Field } from '../marketplace/pieces'
import type { ChannelsStore } from '../../store/channels-store'
import { ChannelMemory } from './ChannelMemory'
import { ChannelIconPicker } from './ChannelIconPicker'

interface ChatDraft {
  chat: string
  botId: string
  allowed: string
}

/**
 * Signal through a signal-cli daemon the person runs on this computer. The daemon owns the linked
 * number; the host only calls its JSON-RPC endpoint, and the number stays in this computer's vault.
 */
export function SignalChannel({ status, model, bots }: {
  status: SignalStatus | null
  model: ChannelsStore
  bots: { id: string; label: string }[]
}) {
  const tr = useTr()
  const [editing, setEditing] = useState(false)
  const [number, setNumber] = useState('')
  const [daemon, setDaemon] = useState('http://127.0.0.1:7583')
  const [chats, setChats] = useState<ChatDraft[]>([{ chat: '', botId: '', allowed: '' }])
  const update = (index: number, change: Partial<ChatDraft>) =>
    setChats((rows) => rows.map((row, i) => (i === index ? { ...row, ...change } : row)))
  const clean = chats
    .map((row) => ({
      chat: row.chat.trim(),
      botId: row.botId || bots[0]?.id || '',
      allowedUsers: row.allowed.split(/[\s,]+/).filter(Boolean),
    }))
    .filter((row) => row.chat || row.allowedUsers.length > 0)
  const ready = signalNumberOk(number.trim())
    && /^https?:\/\/[^\s/]+$/.test(daemon.trim())
    && clean.length > 0
    && clean.every((row) => signalChatOk(row.chat) && row.botId && row.allowedUsers.length > 0
      && row.allowedUsers.every(signalPersonOk))
  const save = async () => {
    const ok = await model.setSignal({ number: number.trim(), daemon: daemon.trim(), bindings: clean })
    if (ok) setEditing(false)
  }
  const start = () => {
    setNumber(status?.number ?? '')
    setDaemon(status?.daemon ?? 'http://127.0.0.1:7583')
    setChats((status?.bindings ?? []).map((binding) => ({
      chat: binding.chat, botId: binding.botId, allowed: binding.allowedUsers.join(', '),
    })))
    if (!status?.bindings?.length) setChats([{ chat: '', botId: '', allowed: '' }])
    setEditing(true)
  }

  if (status?.configured && !editing) {
    return (
      <VStack gap={4} className="settings-stack">
        <Text>{tr('Signal')} — {tr(signalStateText(status.state))}</Text>
        <Text size="caption" tone="tertiary">
          {status.number} — {status.daemon} — {status.bindings?.length ?? 0} {tr('chats bound')}
        </Text>
        {status.bindings?.map((binding) => (
          <VStack key={binding.chat} gap={4} className="settings-stack">
            <Text size="caption" tone="tertiary">
              {binding.chat} → {bots.find((b) => b.id === binding.botId)?.label ?? tr('Unknown bot')} — {binding.allowedUsers.length} {tr('people allowed')}
            </Text>
            <ChannelIconPicker value={binding.icon ?? ''} save={(icon) => model.setIcon('signal', binding.chat, icon)} />
            <ChannelMemory value={binding.memory ?? ''} save={(memory) => model.setMemory('signal', binding.chat, memory)} />
          </VStack>
        ))}
        <HStack gap={8}>
          <Button kind="secondary" disabled={model.busy} onClick={start}>{tr('Change Signal settings')}</Button>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.disconnectSignal()}>{tr('Disconnect Signal')}</Button>
        </HStack>
        {status.state.startsWith('notRunning') && status.setup ? <Text role="status">{status.setup}</Text> : null}
      </VStack>
    )
  }

  return (
    <VStack gap={6} className="settings-stack">
      <Text size="caption" tone="tertiary">
        {tr('Signal needs a signal-cli daemon on this computer. Link the number once with signal-cli -a +15551234567 link, then run signal-cli -a +15551234567 daemon --http 127.0.0.1:7583 --receive-mode=on-start. The number and keys stay on this computer.')}
      </Text>
      {status?.setup ? <Text role="status">{status.setup}</Text> : null}
      <Named label="Signal number"><Field value={number} onChange={setNumber} placeholder="+15551234567" /></Named>
      <Named label="signal-cli daemon"><Field value={daemon} onChange={setDaemon} placeholder="http://127.0.0.1:7583" /></Named>
      {chats.map((row, index) => (
        <VStack key={index} gap={4} className="settings-stack">
          <Named label="Phone number or group"><Field value={row.chat} onChange={(value) => update(index, { chat: value })} placeholder="+15551234567 or group:<id>" /></Named>
          <ChoicePicker
            selection={row.botId || bots[0]?.id || ''}
            options={bots.map((b) => ({ id: b.id, label: b.label }))}
            onChange={(botId) => update(index, { botId })}
          />
          <Named label="Allowed Signal numbers"><Field value={row.allowed} onChange={(value) => update(index, { allowed: value })} placeholder="+15557654321, or a UUID" /></Named>
          <Button kind="secondary" onClick={() => setChats((list) => list.filter((_, i) => i !== index))}>{tr('Remove chat')}</Button>
        </VStack>
      ))}
      <HStack gap={8}>
        <Button kind="secondary" onClick={() => setChats((list) => [...list, { chat: '', botId: '', allowed: '' }])}>{tr('Add chat')}</Button>
        <Button kind="secondary" disabled={model.busy || !ready} onClick={() => void save()}>{tr('Connect Signal')}</Button>
        {status?.configured ? <Button kind="secondary" disabled={model.busy} onClick={() => setEditing(false)}>{tr('Cancel')}</Button> : null}
      </HStack>
    </VStack>
  )
}

function Named({ label, children }: { label: string; children: ReactNode }) {
  return <VStack gap={3}><Text size="caption" tone="secondary">{label}</Text>{children}</VStack>
}
