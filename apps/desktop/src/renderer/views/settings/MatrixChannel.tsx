import { useState, type ReactNode } from 'react'
import { Button, ChoicePicker } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { matrixHomeserverOk, matrixRoomOk, matrixStateText, matrixUserOk, type MatrixStatus } from '../../../shared/channels'
import { useTr } from '../../lib/use-language'
import { Field } from '../marketplace/pieces'
import type { ChannelsStore } from '../../store/channels-store'
import { ChannelMemory } from './ChannelMemory'
import { ChannelIconPicker } from './ChannelIconPicker'

interface RoomDraft {
  roomId: string
  botId: string
  allowed: string
}

/** Matrix is set up on the computer: its session stays there, never in the app. */
export function MatrixChannel({ status, model, bots }: {
  status: MatrixStatus | null
  model: ChannelsStore
  bots: { id: string; label: string }[]
}) {
  const tr = useTr()
  const [editing, setEditing] = useState(false)
  const [homeserver, setHomeserver] = useState('')
  const [user, setUser] = useState('')
  const [token, setToken] = useState('')
  const [password, setPassword] = useState('')
  const [rooms, setRooms] = useState<RoomDraft[]>([{ roomId: '', botId: '', allowed: '' }])
  const update = (index: number, change: Partial<RoomDraft>) =>
    setRooms((rows) => rows.map((row, i) => (i === index ? { ...row, ...change } : row)))
  const clean = rooms
    .map((row) => ({
      roomId: row.roomId.trim(),
      botId: row.botId || bots[0]?.id || '',
      allowedUsers: row.allowed.split(/[\s,]+/).filter(Boolean),
    }))
    .filter((row) => row.roomId || row.allowedUsers.length > 0)
  const ready = matrixHomeserverOk(homeserver.trim())
    && matrixUserOk(user.trim())
    && clean.length > 0
    && clean.every((row) => matrixRoomOk(row.roomId) && row.botId && row.allowedUsers.length > 0
      && row.allowedUsers.every((id) => matrixUserOk(id)))
    && (token.trim().length > 0 || password.length > 0 || status?.configured === true)
  const save = async () => {
    const ok = await model.setMatrix({
      homeserver: homeserver.trim(), user: user.trim(), accessToken: token.trim(), password, bindings: clean,
    })
    if (ok) { setEditing(false); setToken(''); setPassword('') }
  }
  const start = () => {
    setHomeserver(status?.homeserver ?? '')
    setUser(status?.userId ?? '')
    setToken('')
    setPassword('')
    setRooms((status?.bindings ?? []).map((binding) => ({
      roomId: binding.roomId, botId: binding.botId, allowed: binding.allowedUsers.join(', '),
    })))
    if (!status?.bindings?.length) setRooms([{ roomId: '', botId: '', allowed: '' }])
    setEditing(true)
  }

  if (status?.configured && !editing) {
    return (
      <VStack gap={4} className="settings-stack">
        <Text>{tr('Matrix')} · {tr(matrixStateText(status.state))}</Text>
        <Text size="caption" tone="tertiary">
          {status.userId} · {status.homeserver} · {status.bindings?.length ?? 0} {tr('rooms bound')}
        </Text>
        {status.bindings?.map((binding) => (
          <VStack key={binding.roomId} gap={4} className="settings-stack">
            <Text size="caption" tone="tertiary">
              {binding.roomId} → {bots.find((b) => b.id === binding.botId)?.label ?? tr('Unknown bot')} · {binding.allowedUsers.length} {tr('people allowed')}
            </Text>
            <ChannelIconPicker value={binding.icon ?? ''} save={(icon) => model.setIcon('matrix', binding.roomId, icon)} />
            <ChannelMemory value={binding.memory ?? ''} save={(memory) => model.setMemory('matrix', binding.roomId, memory)} />
          </VStack>
        ))}
        <HStack gap={8}>
          <Button kind="secondary" disabled={model.busy} onClick={start}>{tr('Change Matrix settings')}</Button>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.disconnectMatrix()}>{tr('Disconnect Matrix')}</Button>
        </HStack>
      </VStack>
    )
  }

  return (
    <VStack gap={6} className="settings-stack">
      <Text size="caption" tone="tertiary">
        {tr('Make a Matrix account for the bot, invite it to the rooms, and paste its homeserver and credentials here. Leave the access token empty to sign in with the password once: the host keeps the session.')}
      </Text>
      <Named label="Matrix homeserver"><Field value={homeserver} onChange={setHomeserver} placeholder="https://matrix.example.org" /></Named>
      <Named label="Matrix user"><Field value={user} onChange={setUser} placeholder="@bot:example.org" /></Named>
      <Named label="Access token"><Field secret value={token} onChange={setToken} placeholder={tr('Leave empty to keep the saved session')} /></Named>
      <Named label="Password"><Field secret value={password} onChange={setPassword} placeholder={tr('Used once to sign in, never saved')} /></Named>
      {rooms.map((row, index) => (
        <VStack key={index} gap={4} className="settings-stack">
          <Named label="Room id or alias"><Field value={row.roomId} onChange={(value) => update(index, { roomId: value })} placeholder="!room:example.org" /></Named>
          <ChoicePicker
            selection={row.botId || bots[0]?.id || ''}
            options={bots.map((b) => ({ id: b.id, label: b.label }))}
            onChange={(botId) => update(index, { botId })}
          />
          <Named label="Allowed Matrix users">
            <Field value={row.allowed} onChange={(allowed) => update(index, { allowed })} placeholder="@ana:example.org, @beto:example.org" />
          </Named>
          {rooms.length > 1 ? (
            <Button kind="secondary" onClick={() => setRooms((list) => list.filter((_, i) => i !== index))}>{tr('Remove room')}</Button>
          ) : null}
        </VStack>
      ))}
      <HStack gap={8}>
        <Button kind="secondary" onClick={() => setRooms((list) => [...list, { roomId: '', botId: '', allowed: '' }])}>{tr('Add room')}</Button>
        <Button disabled={!ready || model.busy} onClick={() => void save()}>{model.busy ? tr('Signing in…') : tr('Connect Matrix')}</Button>
        {status?.configured ? <Button kind="secondary" disabled={model.busy} onClick={() => setEditing(false)}>{tr('Cancel')}</Button> : null}
      </HStack>
    </VStack>
  )
}

function Named({ label, children }: { label: string; children: ReactNode }) {
  const tr = useTr()
  return <VStack gap={3}><Text size="caption" tone="secondary">{tr(label)}</Text>{children}</VStack>
}
