import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'

interface Feed {
  id: string
  name: string
  url: string
  fetchedAt?: number | null
}

interface Event {
  feed: string
  title: string
  start: number
  allDay: boolean
  location?: string | null
}

function when(event: Event): string {
  const date = new Date(event.start)
  const day = date.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
  return event.allDay ? day : `${day}, ${date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`
}

/** Read-only `.ics` calendars this computer subscribes to, and what they hold next. */
export function CalendarFeedsSection() {
  const app = useApp()
  const local = useModel(app.local)
  const running = local !== null && local.connection.kind === 'online'
  const [feeds, setFeeds] = useState<Feed[]>([])
  const [events, setEvents] = useState<Event[]>([])
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useMemo(() => async () => {
    if (!local) return
    try {
      const client = await local.ready()
      const [list, agenda] = await Promise.all([
        client.call<{ feeds: Feed[] }>('calendarFeeds', {}),
        client.call<{ events: Event[] }>('calendarAgenda', { days: 14 }),
      ])
      setFeeds(list.feeds)
      setEvents(agenda.events)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }, [local])

  useEffect(() => {
    if (running) void load()
  }, [running, load])

  const add = async () => {
    if (!local || !url.trim()) return
    setBusy(true)
    try {
      const client = await local.ready()
      await client.call('addCalendarFeed', { name: name.trim() || 'Calendar', url: url.trim() }, 45_000)
      setName('')
      setUrl('')
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  const remove = async (feed: Feed) => {
    if (!local) return
    try {
      await (await local.ready()).call('removeCalendarFeed', { id: feed.id })
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  return (
    <CardSection
      title="Calendars"
      footer="Paste the .ics address from Google, Apple or Outlook. Codync only reads it; events appear for you and your bots."
    >
      {!running ? (
        <Text size="caption" tone="secondary">Start this computer’s host to read its calendars.</Text>
      ) : (
        <VStack gap={10}>
          <VStack gap={6}>
            <HStack gap={8} align="end" wrap>
              <VStack gap={4} grow>
                <Text size="caption" tone="secondary">Name (optional)</Text>
                <Field label="Name" value={name} onChange={setName} placeholder="Work" />
              </VStack>
              <VStack gap={4} grow>
                <Text size="caption" tone="secondary">Calendar address</Text>
                <Field label="Address" value={url} onChange={setUrl} placeholder="https://example.com/calendar.ics" />
              </VStack>
              <Button kind="secondary" disabled={!url.trim() || busy} onClick={() => void add()}>
                Add
              </Button>
            </HStack>
            {feeds.length ? (
              <VStack gap={4}>
                {feeds.map((feed) => (
                  <HStack key={feed.id} gap={8} align="center" className="scheduled-row">
                    <VStack gap={2} grow>
                      <Text weight="semibold">{feed.name}</Text>
                      <Text size="caption" tone="secondary" className="scheduled-text">{feed.url}</Text>
                    </VStack>
                    <Button kind="secondary" disabled={busy} onClick={() => void remove(feed)}>
                      Remove
                    </Button>
                  </HStack>
                ))}
              </VStack>
            ) : (
              <Text size="caption" tone="secondary">No calendars yet.</Text>
            )}
          </VStack>
          <VStack gap={4}>
            <Text size="callout" weight="semibold">Next two weeks</Text>
            {events.length ? (
              events.slice(0, 40).map((event) => (
                <VStack key={`${event.feed}-${event.start}-${event.title}`} gap={1}>
                  <Text>{event.title}</Text>
                  <Text size="caption" tone="secondary">
                    {when(event)}{event.location ? ` · ${event.location}` : ''}
                  </Text>
                </VStack>
              ))
            ) : (
              <Text size="caption" tone="secondary">No events in the next two weeks.</Text>
            )}
          </VStack>
          {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        </VStack>
      )}
    </CardSection>
  )
}
