import { useEffect, useState } from 'react'
import { Button, CardSection, ChoicePicker } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore, WorkspaceMember } from '../../store/maintenance-store'
import { Field } from '../marketplace/pieces'
import { relativeTime } from '../thread/chat-items'

const ROLES = [
  { id: 'admin', label: 'Admin' },
  { id: 'member', label: 'Member' },
  { id: 'viewer', label: 'Viewer' },
]

/** The people who share this computer: roles, an invite with a one-time token, and removal. */
export function MembersSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  const model = useModel(store)
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [role, setRole] = useState('member')
  useEffect(() => {
    if (online) void store.loadMembers().then(() => store.loadDevices())
  }, [online, store])
  const invite = () => {
    if (!name.trim()) return
    void store.inviteMember(name.trim(), email.trim(), role).then(() => {
      setName('')
      setEmail('')
    })
  }
  return (
    <CardSection title="Members"
      footer="The owner is this computer. Admins and members can work with the bots; viewers read. An invite returns a token once: copy it and give it to that person.">
      {model.members.length === 0 ? <Text size="caption" tone="secondary">Loading…</Text> : null}
      {model.members.map((member) => (
        <MemberRow
          key={member.id}
          member={member}
          online={online}
          busy={store.busy}
          onRole={(next) => void store.setMemberRole(member.id, next)}
          onRemove={() => void store.removeMember(member.id)}
        />
      ))}
      {model.devices.length ? (
        <VStack gap={6}>
          <Text size="callout" weight="semibold">Devices</Text>
          <Text size="caption" tone="secondary">A paired phone works as the person it is tied to. Changing the role applies on its next check.</Text>
          {model.devices.map((device) => (
            <HStack key={device.key} gap={8} align="center" className="scheduled-row">
              <VStack gap={2} grow>
                <Text weight="semibold">{device.name}</Text>
                <Text size="caption" tone="secondary">
                  {device.platform}{device.lastSeenAt ? ` · seen ${relativeTime.day(device.lastSeenAt)}` : ''}
                </Text>
              </VStack>
              <ChoicePicker
                selection={device.memberId ?? ''}
                options={[{ id: '', label: 'No one' }, ...model.members.map((member) => ({ id: member.id, label: member.name }))]}
                onChange={(id) => void store.assignDeviceMember(device.key, id || null)}
              />
            </HStack>
          ))}
        </VStack>
      ) : null}
      <VStack gap={6}>
        <Text size="callout" weight="semibold">Invite someone</Text>
        <HStack gap={8} align="end" wrap>
          <VStack gap={4} grow>
            <Text size="caption" tone="secondary">Name</Text>
            <Field label="Name" value={name} onChange={setName} placeholder="Ada Lovelace" />
          </VStack>
          <VStack gap={4} grow>
            <Text size="caption" tone="secondary">Email (optional)</Text>
            <Field label="Email" value={email} onChange={setEmail} placeholder="ada@example.com" />
          </VStack>
          <ChoicePicker selection={role} options={ROLES} onChange={setRole} />
          <Button kind="secondary" disabled={!online || store.busy || !name.trim()} onClick={invite}>Invite</Button>
        </HStack>
        {model.memberToken ? (
          <HStack gap={6} align="center">
            <Text size="caption" tone="secondary" className="scheduled-text">Token: {model.memberToken}</Text>
            <Button kind="secondary" onClick={() => void window.codync.app.copy(model.memberToken ?? '')}>Copy</Button>
          </HStack>
        ) : null}
      </VStack>
      {store.notice ? <Text size="caption" tone="secondary">{store.notice}</Text> : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
    </CardSection>
  )
}

function MemberRow({ member, online, busy, onRole, onRemove }: {
  member: WorkspaceMember
  online: boolean
  busy: boolean
  onRole: (role: string) => void
  onRemove: () => void
}) {
  const owner = member.role === 'owner'
  return (
    <HStack gap={8} align="center" className="scheduled-row">
      <VStack gap={2} grow>
        <Text weight="semibold">{member.name}</Text>
        <Text size="caption" tone="secondary">
          {member.email || 'No email'}{member.lastSeenAt ? ` · seen ${relativeTime.day(member.lastSeenAt)}` : ''}
        </Text>
      </VStack>
      {owner ? (
        <Text size="caption" tone="secondary">Owner</Text>
      ) : (
        <ChoicePicker selection={member.role} options={ROLES} onChange={onRole} />
      )}
      <Button kind="secondary" disabled={owner || !online || busy} onClick={onRemove}>Remove</Button>
    </HStack>
  )
}
