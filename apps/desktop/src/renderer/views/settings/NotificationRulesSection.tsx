import { useEffect, useState } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore, NotificationRules } from '../../store/maintenance-store'
import { SwitchRow } from '../bots/parts'
import { Field } from '../marketplace/pieces'

const hour = (value: string, fallback: number): number => {
  const parsed = Number.parseInt(value.replace(/[\s,]/g, ''), 10)
  return Number.isFinite(parsed) ? parsed : fallback
}

function when(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { weekday: 'short', hour: '2-digit', minute: '2-digit', month: 'short', day: 'numeric' })
}

/**
 * The alerts that reach the phone, when they may arrive, and the messages waiting to go out.
 * An approval always arrives: a bot waiting for an answer is stuck until the person reads it.
 */
export function NotificationRulesSection({ store, online, nameOf }: {
  store: MaintenanceStore
  online: boolean
  nameOf: (botId: string) => string
}) {
  const model = useModel(store)
  const [draft, setDraft] = useState<NotificationRules | null>(null)
  const [quietStart, setQuietStart] = useState('0')
  const [quietEnd, setQuietEnd] = useState('0')
  const [zone, setZone] = useState('')

  // The host's rules fill the form the first time, and again after a save.
  useEffect(() => {
    if (!model.rules || draft) return
    setDraft(model.rules)
    setQuietStart(String(model.rules.quietStart))
    setQuietEnd(String(model.rules.quietEnd))
    setZone(model.rules.timezone ?? '')
  }, [model.rules, draft])

  const start = hour(quietStart, 0)
  const end = hour(quietEnd, 0)
  const valid = start >= 0 && start <= 23 && end >= 0 && end <= 23
  const save = () => {
    if (!draft) return
    void store.saveRules({ ...draft, quietStart: start, quietEnd: end, timezone: zone.trim() || null })
  }
  type AlertKey = 'done' | 'needsInput' | 'failed' | 'paused'
  const set = (key: AlertKey, on: boolean) => setDraft((current) => (current ? { ...current, [key]: on } as NotificationRules : current))

  return (
    <>
      <CardSection title="Notifications"
        footer="Quiet hours use this computer's time zone unless you name another one (for example Europe/Madrid). Approvals still arrive during quiet hours: a bot waiting for you cannot go on.">
        {draft ? (
          <>
            <SwitchRow title="When a task finishes" detail="The bot is done and its answer is ready." on={draft.done} onChange={(on) => set('done', on)} />
            <SwitchRow title="When a bot needs you" detail="The bot is blocked until you answer. Always arrives, even in quiet hours." on={draft.needsInput} onChange={(on) => set('needsInput', on)} />
            <SwitchRow title="When a task fails" detail="The turn ended with an error." on={draft.failed} onChange={(on) => set('failed', on)} />
            <SwitchRow title="When a task pauses" detail="The bot stopped before it finished." on={draft.paused} onChange={(on) => set('paused', on)} />
            <HStack gap={10}>
              <VStack gap={4} grow>
                <Text size="caption" tone="secondary">Quiet hours start (0–23)</Text>
                <Field label="Quiet hours start" value={quietStart} onChange={setQuietStart} placeholder="22" />
              </VStack>
              <VStack gap={4} grow>
                <Text size="caption" tone="secondary">Quiet hours end (0–23)</Text>
                <Field label="Quiet hours end" value={quietEnd} onChange={setQuietEnd} placeholder="8" />
              </VStack>
            </HStack>
            <VStack gap={4}>
              <Text size="caption" tone="secondary">Time zone for quiet hours</Text>
              <Field label="Time zone" value={zone} onChange={setZone} placeholder="This computer's" />
            </VStack>
            <HStack gap={8}>
              <Button disabled={!online || store.busy || !valid} onClick={save}>Save rules</Button>
              <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.loadNotifications()}>Reload</Button>
            </HStack>
          </>
        ) : (
          <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.loadNotifications()}>Load rules</Button>
        )}
      </CardSection>

      <CardSection title="Scheduled messages"
        footer="A scheduled message is sent as yours at that time, and the bot answers it then. Messages go out when this computer's host is running; one whose time passed while it was off goes out as soon as it starts.">
        {model.scheduled.length === 0 ? (
          <Text size="caption" tone="secondary">Nothing is waiting to go out.</Text>
        ) : (
          model.scheduled.map((message) => (
            <VStack key={message.id} gap={4} className="scheduled-row">
              <HStack gap={8} align="center">
                <Text weight="semibold">{nameOf(message.botId)}</Text>
                <Text size="caption" tone="secondary">{when(message.sendAt)}</Text>
              </HStack>
              <Text size="caption" tone="secondary" className="scheduled-text">{message.text}</Text>
              <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.cancelScheduled(message.id)}>Cancel</Button>
            </VStack>
          ))
        )}
        <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.loadNotifications()}>Reload</Button>
      </CardSection>
      {store.notice ? <Text size="caption" tone="secondary">{store.notice}</Text> : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
    </>
  )
}
