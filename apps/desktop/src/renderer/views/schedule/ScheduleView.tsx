import { useEffect, useMemo, useRef, useState } from 'react'
import { Button, IconButton, SegmentedChoice } from '../../components/Controls'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { errorText } from '../routines/routine'
import { currentZone } from '../routines/time-zones'
import { ScheduleCreateDialog, type ScheduleBot, type ScheduleSlot } from './ScheduleCreateDialog'
import {
  dayOf, dayLabel, hourOf, rangeFor, rangeLabel, scheduleRows, shiftAnchor, timeOf, today,
  type ScheduleListing, type ScheduleMode, type ScheduleRow,
} from './schedule'
import './schedule.css'

const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
const CHIP_LIMIT: Record<ScheduleMode, number> = { month: 3, week: 5, day: 24 }
const browserZone = () => currentZone(Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC')

/** A calendar of every bot's scheduled routines, with month, week and day views. */
export function ScheduleView({ botId, bots }: { botId: string; bots: ScheduleBot[] }) {
  const tr = useTr()
  const store = useStore()
  const [mode, setMode] = useState<ScheduleMode>('month')
  const [anchor, setAnchor] = useState(() => today(browserZone()))
  const [zone, setZone] = useState(browserZone)
  const [listing, setListing] = useState<ScheduleListing | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [slot, setSlot] = useState<ScheduleSlot | null>(null)
  const range = useMemo(() => rangeFor(anchor, mode), [anchor, mode])
  const current = useMemo(() => today(zone), [zone])
  const reload = useRef<() => void>(() => {})

  useEffect(() => {
    let stopped = false
    let timer: ReturnType<typeof setTimeout> | undefined
    const tick = async () => {
      try {
        const result = await (await store.ready()).call<ScheduleListing>('scheduleCalendar', { from: range.from, to: range.to })
        if (!stopped) {
          setListing(result)
          setError(null)
        }
      } catch (cause) {
        if (!stopped && !store.isOffline) setError(errorText(cause))
      }
      if (!stopped) timer = setTimeout(() => void tick(), 30_000)
    }
    reload.current = () => void tick()
    void tick()
    return () => {
      stopped = true
      clearTimeout(timer)
    }
  }, [range.from, range.to, store])

  const byDay = useMemo(() => {
    const map = new Map<string, ScheduleRow[]>()
    for (const row of scheduleRows(listing?.items ?? [], zone)) {
      const key = dayOf(row.item.at, zone)
      const list = map.get(key) ?? []
      list.push(row)
      map.set(key, list)
    }
    return map
  }, [listing, zone])

  const move = (delta: number) => setAnchor((day) => shiftAnchor(day, mode, delta))
  const goToday = () => setAnchor(today(zone))
  const canCreate = (day: string, hour: number | null) => {
    if (day < current) return false
    return hour === null || day > current || hour >= hourOf(Date.now(), zone)
  }
  const open = (day: string, hour: number | null) => {
    if (canCreate(day, hour)) setSlot({ day, hour })
  }
  const chip = (row: ScheduleRow) => {
    const { item } = row
    return (
      <span key={row.key} className={`schedule-chip ${item.trigger}${row.paused ? ' paused' : ''}`} title={`${item.name} · ${item.description ?? ''}`}>
        <span className="schedule-chip-time">{row.paused ? tr('Paused') : timeOf(item.at, zone)}</span>
        <span className="schedule-chip-name">{item.name}</span>
      </span>
    )
  }
  const chips = (day: string) => (byDay.get(day) ?? []).slice(0, CHIP_LIMIT[mode]).map(chip)
  const more = (day: string) => Math.max(0, (byDay.get(day)?.length ?? 0) - CHIP_LIMIT[mode])

  const stepTitle = (direction: 'Previous' | 'Next') => tr(`${direction} ${mode === 'month' ? 'month' : mode === 'week' ? 'week' : 'day'}`)
  const viewOptions: { id: ScheduleMode; label: string }[] = [
    { id: 'month', label: 'Month' },
    { id: 'week', label: 'Week' },
    { id: 'day', label: 'Day' },
  ]

  return (
    <VStack gap={12} className="schedule">
      <HStack gap={8} className="schedule-toolbar">
        <IconButton title={stepTitle('Previous')} icon="chevron.left" onClick={() => move(-1)} />
        <IconButton title={stepTitle('Next')} icon="chevron.right" onClick={() => move(1)} />
        <Button kind="secondary" onClick={goToday}>{tr('Today')}</Button>
        <Text size="body" weight="semibold" grow className="schedule-range">{rangeLabel(anchor, mode)}</Text>
        <SegmentedChoice selection={mode} options={viewOptions} onChange={setMode} />
      </HStack>
      {error ? <Text size="caption" tone="danger">{error}</Text> : null}
      {!listing && !error ? <Text size="callout" tone="secondary">{tr('Loading schedule…')}</Text> : null}
      {listing && listing.items.length === 0 ? (
        <Text as="div" size="callout" tone="secondary" className="outlined-card schedule-empty">
          {tr('No routines are scheduled yet. Right-click a day, or click an hour in Day view, to create one.')}
        </Text>
      ) : null}
      <div key={`${mode}-${anchor}-${zone}`} className="schedule-fade">
        {mode === 'day' ? (
          <div className="schedule-hours" role="grid" aria-label={dayLabel(anchor)}>
            <Text size="callout" weight="medium" tone="secondary" className="schedule-day-title">{dayLabel(anchor)}</Text>
            {Array.from({ length: 24 }, (_, hour) => (
              <div
                key={hour}
                className={`schedule-hour${canCreate(anchor, hour) ? ' open' : ''}`}
                role="row"
                onClick={(event) => {
                  if ((event.target as HTMLElement).closest('.schedule-chip')) return
                  open(anchor, hour)
                }}
              >
                <Text size="caption" tone="tertiary" className="schedule-hour-label">{String(hour).padStart(2, '0')}:00</Text>
                <div className="schedule-hour-items">{(byDay.get(anchor) ?? []).filter((row) => hourOf(row.item.at, zone) === hour).map(chip)}</div>
              </div>
            ))}
          </div>
        ) : (
          <div className="schedule-grid" data-view={mode} role="grid" aria-label={rangeLabel(anchor, mode)}>
            {range.days.slice(0, 7).map((day) => (
              <Text key={day} size="caption" tone="tertiary" className="schedule-weekday" aria-hidden>{tr(WEEKDAYS[(new Date(`${day}T00:00:00Z`).getUTCDay() + 6) % 7] ?? '')}</Text>
            ))}
            {range.days.map((day) => {
              const outside = mode === 'month' && day.slice(0, 7) !== anchor.slice(0, 7)
              const currentDay = day === current
              return (
                <div
                  key={day}
                  className={`schedule-cell${outside ? ' outside' : ''}${currentDay ? ' today' : ''}${canCreate(day, null) ? ' open' : ''}`}
                  role="row"
                  aria-label={dayLabel(day)}
                  onContextMenu={(event) => {
                    event.preventDefault()
                    if ((event.target as HTMLElement).closest('.schedule-chip')) return
                    open(day, null)
                  }}
                >
                  <button className="schedule-day-button" onClick={() => { setAnchor(day); setMode('day') }}>
                    {new Date(`${day}T00:00:00Z`).toLocaleDateString(undefined, { timeZone: 'UTC', day: 'numeric' })}
                  </button>
                  <div className="schedule-cell-items">{chips(day)}</div>
                  {more(day) > 0 ? (
                    <button className="schedule-more" onClick={() => { setAnchor(day); setMode('day') }}>{tr('+{count} more').replaceAll('{count}', () => String(more(day)))}</button>
                  ) : null}
                </div>
              )
            })}
          </div>
        )}
      </div>
      <HStack gap={8} className="schedule-hint">
        <Text size="caption" tone="tertiary">{tr('Time zone: {zone}').replaceAll('{zone}', () => zone.replaceAll('_', ' '))}</Text>
        <Spacer />
        {bots.length === 0 ? <Text size="caption" tone="tertiary">{tr('Create a bot to schedule a routine.')}</Text> : null}
      </HStack>
      {slot && bots.length > 0 ? (
        <ScheduleCreateDialog
          slot={slot}
          zone={zone}
          onZone={(picked) => setZone(picked)}
          bots={bots}
          botId={botId}
          onClose={() => setSlot(null)}
          onCreated={() => {
            setSlot(null)
            setAnchor(slot.day)
            reload.current()
          }}
        />
      ) : null}
    </VStack>
  )
}
