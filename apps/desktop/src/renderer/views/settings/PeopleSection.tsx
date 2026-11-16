import { useEffect, useState } from 'react'
import type { AuthorizedDevice, MemberInvite, MemberSession, Person } from '@shared/models'
import { Button, CardSection, ChoicePicker } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'
import { MemberSignInSheet } from './MemberSignInSheet'

/**
 * Who is on this computer besides its owner (see docs/features/people.md). A person is a member
 * account without a password of its own: an invitation hands out a one-time code, and the device or
 * session it becomes acts as that person. Every method here is loopback-only and refused to a
 * member's own phone, which the host checks.
 */
export function PeopleSection() {
  const tr = useTr()
  const app = useApp()
  const [people, setPeople] = useState<Person[]>([])
  const [sessions, setSessions] = useState<MemberSession[]>([])
  const [canManageRoles, setCanManageRoles] = useState(true)
  const [devices, setDevices] = useState<AuthorizedDevice[]>([])
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [inviteName, setInviteName] = useState('')
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteRole, setInviteRole] = useState<'member' | 'admin'>('member')
  const [issued, setIssued] = useState<{ invite: MemberInvite; person: Person } | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [removing, setRemoving] = useState<Person | null>(null)
  const [signingIn, setSigningIn] = useState(false)

  const client = async () => {
    const local = app.local
    return local ? await local.ready() : null
  }
  const load = async () => {
    const c = await client()
    if (!c) return
    // An older host has no people or member sessions: those sections simply stay empty.
    const [p, d, s] = await Promise.allSettled([
      c.call<{ people: Person[]; canManageRoles?: boolean }>('people'),
      c.call<{ devices: AuthorizedDevice[] }>('devices'),
      c.call<{ sessions: MemberSession[] }>('memberSessions'),
    ])
    if (p.status === 'fulfilled') {
      setPeople(p.value.people ?? [])
      setCanManageRoles(p.value.canManageRoles ?? true)
    }
    if (d.status === 'fulfilled') setDevices(d.value.devices ?? [])
    setSessions(s.status === 'fulfilled' ? (s.value.sessions ?? []) : [])
  }
  useEffect(() => {
    void load()
  }, [app])

  const add = async () => {
    const c = await client()
    if (!c) return
    setBusy(true)
    try {
      await c.call('addPerson', { name: name.trim(), email: email.trim() })
      setName('')
      setEmail('')
      setError(null)
      await load()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    setBusy(false)
  }
  const invite = async () => {
    const c = await client()
    if (!c) return
    setBusy(true)
    try {
      const answer = await c.call<{ person: Person; invite: MemberInvite }>('inviteMember', {
        name: inviteName.trim(),
        email: inviteEmail.trim(),
        role: inviteRole,
      })
      setIssued(answer)
      setInviteName('')
      setInviteEmail('')
      setInviteRole('member')
      setError(null)
      await load()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    setBusy(false)
  }
  const revoke = async (id: string) => {
    const c = await client()
    if (!c) return
    try {
      await c.call('revokeMemberSession', { session: id })
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    await load()
  }
  const remove = async (person: Person) => {
    const c = await client()
    if (!c) return
    try {
      await c.call('removePerson', { id: person.id })
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    await load()
  }
  const assign = async (device: AuthorizedDevice, person: string) => {
    const c = await client()
    if (!c) return
    try {
      await c.call('setDevicePerson', { device: device.key, person: person || null })
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    await load()
  }
  const setRole = async (person: Person, role: 'admin' | 'member') => {
    const c = await client()
    if (!c) return
    try {
      await c.call('updatePerson', { id: person.id, name: person.name, email: person.email ?? '', role })
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    await load()
  }

  const choices = [
    { id: '', label: tr("This computer's owner") },
    ...people.map((p) => ({ id: p.id, label: p.name })),
  ]
  const roles = [
    { id: 'member', label: tr('Member') },
    { id: 'admin', label: tr('Admin') },
  ]
  const sessionDetail = (session: MemberSession) => {
    if (session.kind === 'invite') {
      const until = session.expiresAt ? new Date(session.expiresAt).toLocaleString() : ''
      return `${tr('Invitation')}${until ? ` · ${until}` : ''}`
    }
    return `${session.name ?? tr('Session')} · ${session.platform ?? ''}`.trim()
  }
  return (
    <>
      <CardSection
        title={tr('People on this computer')}
        footer={tr('Someone who uses this computer with their own phone or their own bots. Their phone sees only their bots, chats and connectors, plus the ones shared with everyone.')}
      >
        {people.length === 0 ? <Text tone="secondary">{tr('No one else yet')}</Text> : null}
        {people.map((person) => (
          <HStack key={person.id} gap={8} className="settings-row settings-appear">
            <VStack grow className="settings-stack">
              <span>{person.name}</span>
              {person.email ? <Text size="caption" tone="tertiary">{person.email}</Text> : null}
              <Text size="caption" tone="tertiary">
                {person.role === 'admin'
                  ? tr('Admins manage people, connectors and bots on this computer.')
                  : tr('Members see only their own bots, chats and connectors.')}
              </Text>
            </VStack>
            {canManageRoles ? (
              <ChoicePicker
                selection={person.role ?? 'member'}
                options={roles}
                onChange={(role) => void setRole(person, role as 'admin' | 'member')}
              />
            ) : (
              <Text size="caption" tone="secondary">{person.role === 'admin' ? tr('Admin') : tr('Member')}</Text>
            )}
            <Button kind="secondary" onClick={() => setRemoving(person)}>{tr('Remove')}</Button>
          </HStack>
        ))}
        <HStack gap={6} wrap>
          <Field label={tr('Name')} value={name} onChange={setName} />
          <Field label={tr('Email (optional)')} value={email} onChange={setEmail} placeholder="person@example.com" />
          <Button disabled={busy || !name.trim()} onClick={() => void add()}>{busy ? tr('Adding…') : tr('Add a person')}</Button>
        </HStack>
      </CardSection>
      <CardSection
        title={tr('Invite a member')}
        footer={tr('An invitation is a one-time code: your phone scans it, or this app signs in with it at this computer. The member acts as that person, with only their own bots, chats and connectors.')}
      >
        <HStack gap={6} wrap>
          <Field label={tr('Name (optional)')} value={inviteName} onChange={setInviteName} />
          <Field label={tr('Email')} value={inviteEmail} onChange={setInviteEmail} placeholder="person@example.com" />
          {canManageRoles ? (
            <ChoicePicker selection={inviteRole} options={roles} onChange={(role) => setInviteRole(role as 'member' | 'admin')} />
          ) : null}
          <Button disabled={busy || !inviteEmail.trim()} onClick={() => void invite()}>
            {busy ? tr('Inviting…') : tr('Invite')}
          </Button>
        </HStack>
        {issued ? (
          <VStack gap={4} className="settings-stack">
            <Text size="caption" tone="secondary">
              {tr('Invitation for {name}').replaceAll('{name}', issued.person.name)}
            </Text>
            <Text size="caption" mono>{issued.invite.code}</Text>
            {issued.invite.pairingUrl ? (
              <HStack gap={6} wrap>
                <Button kind="secondary" onClick={() => window.codync.app.copy(issued.invite.pairingUrl ?? '')}>
                  {tr('Copy invitation link')}
                </Button>
                <Button kind="secondary" onClick={() => window.codync.app.copy(issued.invite.code)}>
                  {tr('Copy code')}
                </Button>
              </HStack>
            ) : null}
            <Text size="caption" tone="tertiary">
              {issued.invite.note ?? tr('The code works once, until {date}.').replaceAll('{date}', new Date(issued.invite.expiresAt).toLocaleString())}
            </Text>
          </VStack>
        ) : null}
        <Button kind="secondary" onClick={() => setSigningIn(true)}>{tr('Sign in as a member on this computer')}</Button>
      </CardSection>
      <CardSection
        title={tr('Member sessions')}
        footer={tr('A session is a paired phone or a signed-in app. Ending one revokes that device at once; the person and their bots stay.')}
      >
        {sessions.length === 0 ? <Text tone="secondary">{tr('None yet')}</Text> : null}
        {sessions.map((session) => (
          <HStack key={session.id} gap={8} className="settings-row settings-appear">
            <VStack grow className="settings-stack">
              <span>{session.person}</span>
              <Text size="caption" tone="tertiary">{sessionDetail(session)}</Text>
              {session.email ? <Text size="caption" tone="tertiary">{session.email}</Text> : null}
            </VStack>
            <Text size="caption" tone="secondary">
              {session.kind === 'invite' ? tr('Pending') : session.connected ? tr('Connected') : tr('Signed out')}
            </Text>
            <Button kind="secondary" onClick={() => void revoke(session.id)}>{tr('End session')}</Button>
          </HStack>
        ))}
      </CardSection>
      <CardSection
        title={tr('Assign a device')}
        footer={tr('A member\'s phone only sees and changes what is theirs.')}
      >
        {devices.length === 0 ? <Text tone="secondary">{tr('None yet')}</Text> : null}
        {devices.map((device) => (
          <HStack key={device.key} gap={8} className="settings-row settings-appear">
            <VStack grow className="settings-stack">
              <span>{device.name}</span>
              <Text size="caption" tone="tertiary">{device.person ?? tr("This computer's owner")}</Text>
            </VStack>
            <ChoicePicker
              selection={device.personId ?? ''}
              options={choices}
              onChange={(id) => void assign(device, id)}
            />
          </HStack>
        ))}
      </CardSection>
      {error ? <Text role="alert" tone="danger">{error}</Text> : null}
      <Dialog
        open={removing !== null}
        title={tr('Remove {name}?').replaceAll('{name}', removing?.name ?? tr('Person'))}
        message={tr('Their phone loses access at once. Their bots, chats and connectors become yours. Nothing is deleted.')}
        actions={removing ? [{ title: tr('Remove'), destructive: true, action: () => void remove(removing) }] : []}
        onClose={() => setRemoving(null)}
      />
      <MemberSignInSheet open={signingIn} onClose={() => setSigningIn(false)} />
    </>
  )
}
