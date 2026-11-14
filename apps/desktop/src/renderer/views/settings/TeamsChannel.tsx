import { useState, type ReactNode } from 'react'
import { Button, ChoicePicker } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { teamsStateText, type TeamsStatus } from '../../../shared/channels'
import { useTr } from '../../lib/use-language'
import { Field } from '../marketplace/pieces'
import type { ChannelsStore } from '../../store/channels-store'
import { ChannelMemory } from './ChannelMemory'
import { ChannelIconPicker } from './ChannelIconPicker'

/**
 * Microsoft Teams through an Azure Bot resource and the Bot Framework connector. The app password
 * stays in this computer's vault; the registration and its messaging endpoint are the person's own
 * Azure work, and the host never pretends a connection it cannot authenticate.
 */
export function TeamsChannel({ status, model, bots }: {
  status: TeamsStatus | null
  model: ChannelsStore
  bots: { id: string; label: string }[]
}) {
  const tr = useTr()
  const [editing, setEditing] = useState(false)
  const [appId, setAppId] = useState('')
  const [tenantId, setTenantId] = useState('')
  const [password, setPassword] = useState('')
  const [managed, setManaged] = useState(false)
  const [botId, setBotId] = useState('')
  const [users, setUsers] = useState('')
  const [tenants, setTenants] = useState('')
  const [memory, setMemory] = useState('')
  const [icon, setIcon] = useState('')
  const userList = users.split(/[\s,]+/).filter(Boolean)
  const tenantList = tenants.split(/[\s,]+/).filter(Boolean)
  const ready = /^[0-9a-fA-F-]{36}$/.test(appId.trim())
    && (managed || password.length > 0 || status?.configured === true)
    && (botId || bots[0]?.id)
    && (userList.length > 0 || tenantList.length > 0)
  const start = () => {
    setAppId(status?.appId ?? '')
    setTenantId(status?.tenantId ?? '')
    setPassword('')
    setManaged(status?.managedIdentity ?? false)
    setBotId(status?.botId ?? '')
    setUsers((status?.allowedUsers ?? []).join(', '))
    setTenants((status?.allowedTenants ?? []).join(', '))
    setMemory(status?.memory ?? '')
    setIcon(status?.icon ?? '')
    setEditing(true)
  }
  const save = async () => {
    const ok = await model.setTeams({
      appId: appId.trim(), tenantId: tenantId.trim(), appPassword: password, managedIdentity: managed,
      botId: botId || bots[0]?.id || '', allowedUsers: userList, allowedTenants: tenantList, memory, icon,
    })
    if (ok) { setEditing(false); setPassword('') }
  }

  if (status?.configured && !editing) {
    return (
      <VStack gap={4} className="settings-stack">
        <Text>{tr('Microsoft Teams')} — {tr(teamsStateText(status.state))}</Text>
        <Text size="caption" tone="tertiary">
          {status.appId} — {bots.find((b) => b.id === status.botId)?.label ?? tr('Unknown bot')} —
          {' '}{status.allowedUsers?.length ?? 0} {tr('people allowed')} — {status.allowedTenants?.length ?? 0} {tr('Tenants allowed')}
        </Text>
        <Text size="caption" tone="tertiary">{tr('Messaging endpoint')}: {status.endpoint}</Text>
        <ChannelIconPicker value={status.icon ?? ''} save={(value) => model.setIcon('teams', 'teams', value)} />
        <ChannelMemory value={status.memory ?? ''} save={(value) => model.setMemory('teams', 'teams', value)} />
        <HStack gap={8}>
          <Button kind="secondary" disabled={model.busy} onClick={start}>{tr('Change Teams settings')}</Button>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.disconnectTeams()}>{tr('Disconnect Teams')}</Button>
        </HStack>
      </VStack>
    )
  }

  return (
    <VStack gap={6} className="settings-stack">
      <Text>{tr('Microsoft Teams')} — {tr(teamsStateText(status?.state ?? 'notConfigured'))}</Text>
      <Text size="caption" tone="tertiary">
        {tr('Create an Azure Bot resource, enable the Microsoft Teams channel, and point its messaging endpoint at this computer. Teams only reaches a public HTTPS address. The client secret stays on this computer.')}
      </Text>
      {status?.registration ? <Text role="status">{status.registration}</Text> : null}
      <Named label="Microsoft app id"><Field value={appId} onChange={setAppId} placeholder="00000000-0000-0000-0000-000000000000" /></Named>
      <Named label="Directory tenant"><Field value={tenantId} onChange={setTenantId} placeholder="contoso.onmicrosoft.com, or empty for multi-tenant" /></Named>
      <ChoicePicker
        selection={managed ? 'managed' : 'secret'}
        options={[{ id: 'secret', label: tr('App client secret') }, { id: 'managed', label: tr('Managed identity') }]}
        onChange={(value) => setManaged(value === 'managed')}
      />
      {!managed ? <Named label="Client secret"><Field secret value={password} onChange={setPassword} placeholder={status?.configured ? tr('Leave empty to keep the saved secret') : tr('The app registration secret')} /></Named> : null}
      <ChoicePicker
        selection={botId || bots[0]?.id || ''}
        options={bots.map((b) => ({ id: b.id, label: b.label }))}
        onChange={setBotId}
      />
      <Named label="People allowed"><Field value={users} onChange={setUsers} placeholder="ana@example.com, or an AAD object id, or *" /></Named>
      <Named label="Tenants allowed"><Field value={tenants} onChange={setTenants} placeholder="a tenant id, or *" /></Named>
      <Named label="What the bot knows about Teams"><Field value={memory} onChange={setMemory} placeholder={tr('Given to the bot before each Teams message')} /></Named>
      <HStack gap={8}>
        <Button kind="secondary" disabled={model.busy || !ready} onClick={() => void save()}>{tr('Connect Teams')}</Button>
        {status?.configured ? <Button kind="secondary" disabled={model.busy} onClick={() => setEditing(false)}>{tr('Cancel')}</Button> : null}
      </HStack>
    </VStack>
  )
}

function Named({ label, children }: { label: string; children: ReactNode }) {
  return <VStack gap={3}><Text size="caption" tone="secondary">{label}</Text>{children}</VStack>
}
