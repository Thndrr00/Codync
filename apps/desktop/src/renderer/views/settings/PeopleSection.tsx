import { useEffect, useState } from 'react'
import type { AuthorizedDevice, Person } from '@shared/models'
import { Button, CardSection, ChoicePicker } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'

/**
 * Who is on this computer besides its owner (see docs/features/people.md). A person is not a member account:
 * they are a label the owner puts on a paired phone or a bot, and the host authorizes every call by it.
 * Every method here is loopback-only, so a member's phone never reaches this section.
 */
export function PeopleSection() {
  const tr = useTr()
  const app = useApp()
  const [people, setPeople] = useState<Person[]>([])
  const [devices, setDevices] = useState<AuthorizedDevice[]>([])
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [removing, setRemoving] = useState<Person | null>(null)

  const client = async () => {
    const local = app.local
    return local ? await local.ready() : null
  }
  const load = async () => {
    const c = await client()
    if (!c) return
    // An older host has no people: the section simply stays empty.
    const [p, d] = await Promise.allSettled([
      c.call<{ people: Person[] }>('people'),
      c.call<{ devices: AuthorizedDevice[] }>('devices'),
    ])
    if (p.status === 'fulfilled') setPeople(p.value.people ?? [])
    if (d.status === 'fulfilled') setDevices(d.value.devices ?? [])
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

  const choices = [
    { id: '', label: tr("This computer's owner") },
    ...people.map((p) => ({ id: p.id, label: p.name })),
  ]
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
            </VStack>
            <Button kind="secondary" onClick={() => setRemoving(person)}>{tr('Remove')}</Button>
          </HStack>
        ))}
        <HStack gap={6} wrap>
          <Field label={tr('Name')} value={name} onChange={setName} />
          <Field label={tr('Email (optional)')} value={email} onChange={setEmail} placeholder="person@example.com" />
          <Button disabled={busy || !name.trim()} onClick={() => void add()}>{busy ? tr('Adding…') : tr('Add a person')}</Button>
        </HStack>
        {error ? <Text role="alert" tone="danger">{error}</Text> : null}
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
      <Dialog
        open={removing !== null}
        title={tr('Remove {name}?').replaceAll('{name}', removing?.name ?? tr('Person'))}
        message={tr('Their phone, bots, chats and connectors become yours. Nothing is deleted.')}
        actions={removing ? [{ title: tr('Remove'), destructive: true, action: () => void remove(removing) }] : []}
        onClose={() => setRemoving(null)}
      />
    </>
  )
}
