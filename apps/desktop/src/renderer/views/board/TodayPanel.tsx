import { useEffect, useState } from 'react'
import { IconButton, Spinner, Button } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import type { BotStore } from '../../store/bot-store'
import { errorText } from '../bots/drafts'
import { Field } from '../marketplace/pieces'

interface Digest {
  text: string
  overdue: string[]
  dueToday: string[]
  inProgress: string[]
  approvals: { bot: string; title: string }[]
  events: { title?: string }[]
  waiting: string[]
}

interface Weather {
  place: string
  text: string
  temperature?: number | null
}

/**
 * The day at a glance, computed by the host from the board, approvals, calendars and waiting items.
 * No model is asked; refreshing only rereads what the computer already has.
 */
export function TodayPanel({ store }: { store: BotStore }) {
  const tr = useTr()
  const [digest, setDigest] = useState<Digest | null>(null)
  const [weather, setWeather] = useState<Weather | null>(null)
  const [place, setPlace] = useState('')
  const [editingPlace, setEditingPlace] = useState(false)
  const [weatherError, setWeatherError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = async () => {
    setLoading(true)
    try {
      const answer = await (await store.ready()).call<Digest>('dailyDigest', {})
      setDigest(answer)
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }

  const loadWeather = async () => {
    try {
      const answer = await (await store.ready()).call<Weather>('weather', {})
      setWeather(answer)
      setWeatherError(null)
    } catch (e) {
      setWeather(null)
      setWeatherError(errorText(e))
    }
  }

  const savePlace = async () => {
    if (!place.trim()) return
    try {
      const answer = await (await store.ready()).call<Weather>(
        'weather',
        { place: place.trim(), save: true },
        30_000,
      )
      setWeather(answer)
      setPlace('')
      setEditingPlace(false)
      setWeatherError(null)
    } catch (e) {
      setWeatherError(errorText(e))
    }
  }

  useEffect(() => {
    void load()
    void loadWeather()
    const timer = setInterval(() => {
      void load()
      void loadWeather()
    }, 60_000)
    return () => clearInterval(timer)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store])

  return (
    <section className="board-today" aria-label={tr('Today')}>
      <HStack gap={8} align="center">
        <Text size="callout" weight="semibold" grow>{tr('Today')}</Text>
        {loading ? <Spinner /> : null}
        <IconButton title={tr('Refresh')} icon="arrow.clockwise" onClick={() => { void load(); void loadWeather() }} />
      </HStack>
      {digest ? (
        <VStack gap={6}>
          <Text size="callout" className="board-today-text">{digest.text}</Text>
          <HStack gap={6} wrap>
            {digest.overdue.length ? <Text size="caption" tone="danger">{tr('Overdue')}: {digest.overdue.length}</Text> : null}
            {digest.dueToday.length ? <Text size="caption" tone="secondary">{tr('Due today')}: {digest.dueToday.length}</Text> : null}
            {digest.approvals.length ? <Text size="caption" tone="secondary">{tr('Approvals')}: {digest.approvals.length}</Text> : null}
            {digest.events.length ? <Text size="caption" tone="secondary">{tr('Events')}: {digest.events.length}</Text> : null}
            {digest.waiting.length ? <Text size="caption" tone="secondary">{tr('Waiting for')}: {digest.waiting.length}</Text> : null}
          </HStack>
        </VStack>
      ) : null}
      {weather ? (
        <HStack gap={6} align="center" wrap>
          <Text size="caption" tone="secondary">{weather.text}</Text>
          <Button kind="secondary" onClick={() => setEditingPlace(true)}>{tr('Change place')}</Button>
        </HStack>
      ) : null}
      {!weather || editingPlace ? (
        <HStack gap={6} align="end" wrap>
          <VStack gap={4} grow>
            <Text size="caption" tone="secondary">{tr('Weather place')}</Text>
            <Field label="Weather place" value={place} onChange={setPlace} placeholder={tr('Madrid')} />
          </VStack>
          <Button kind="secondary" disabled={!place.trim()} onClick={() => void savePlace()}>{tr('Save')}</Button>
        </HStack>
      ) : null}
      {weatherError && !weather ? <Text size="caption" tone="tertiary">{weatherError}</Text> : null}
      {error ? <Text size="caption" tone="danger">{error}</Text> : null}
    </section>
  )
}
