import { ChoicePicker } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { clock, intervalUnits, minuteSteps, scheduleStyles, toggleDay, usesTime, weekdayInitials, weekdayNames, wrap } from '../../lib/routine-schedule'
import { Field } from '../bots/parts'
import type { RoutineScheduleDraft, RoutineSchedulePreview } from './routine'

type Change = (change: Partial<RoutineScheduleDraft>) => void

const hours = Array.from({ length: 24 }, (_, i) => ({ id: i, label: clock(i, 0).slice(0, 2) }))
const minutes = Array.from({ length: 60 }, (_, i) => ({ id: i, label: clock(0, i).slice(3) }))

/**
 * The visual fields of a recurring routine schedule (days, times, zone). The host turns them into a cron
 * expression, validates them and reports the next run; nothing here computes a date.
 */
export function ScheduleFields({ schedule, update, preview, error }: { schedule: RoutineScheduleDraft; update: Change; preview: RoutineSchedulePreview | null; error: string | null }) {
  const style = schedule.calendarStyle
  return (
    <VStack gap={14} className="routine-cron">
      <Field label="Repeats">
        <ChoicePicker selection={style} options={scheduleStyles} onChange={(calendarStyle) => update({ calendarStyle })} fits />
      </Field>
      {style === 'custom' ? (
        <Field label="Cron">
          <input className="routine-input routine-cron-input" value={schedule.expression} placeholder="0 9 * * 1-5" spellCheck={false} autoCorrect="off" onChange={(e) => update({ expression: e.target.value })} />
        </Field>
      ) : null}
      {style === 'minutes' ? (
        <Field label="Every">
          <ChoicePicker selection={schedule.minuteStep} options={minuteSteps.map((id) => ({ id, label: `${id} min` }))} onChange={(minuteStep) => update({ minuteStep })} fits />
        </Field>
      ) : null}
      {style === 'weekly' ? (
        <Field label="Day">
          <ChoicePicker selection={schedule.weekday} options={weekdayNames.map((label, id) => ({ id, label }))} onChange={(weekday) => update({ weekday })} fits />
        </Field>
      ) : null}
      {style === 'days' ? (
        <Field label="Days">
          <HStack gap={6} role="group" aria-label="Days">
            {weekdayInitials.map((initial, day) => {
              const on = schedule.selectedDays.includes(day)
              return (
                <button key={day} className={`press routine-day ${on ? 'on' : ''}`} aria-pressed={on} aria-label={weekdayNames[day]} onClick={() => update({ selectedDays: toggleDay(schedule.selectedDays, day) })}>
                  {initial}
                </button>
              )
            })}
          </HStack>
        </Field>
      ) : null}
      {style === 'monthly' ? (
        <Field label="Day of the month">
          <ChoicePicker selection={schedule.monthDay} options={Array.from({ length: 31 }, (_, i) => ({ id: i + 1, label: String(i + 1) }))} onChange={(monthDay) => update({ monthDay })} fits />
        </Field>
      ) : null}
      {style === 'hourly' ? (
        <Field label="Minute past the hour">
          <ChoicePicker selection={wrap(schedule.minute, 60)} options={minutes} onChange={(minute) => update({ minute })} fits />
        </Field>
      ) : null}
      {usesTime(style) ? (
        <Field label="At">
          <HStack gap={8}>
            <ChoicePicker selection={wrap(schedule.hour, 24)} options={hours} onChange={(hour) => update({ hour })} fits />
            <Text tone="secondary">:</Text>
            <ChoicePicker selection={wrap(schedule.minute, 60)} options={minutes} onChange={(minute) => update({ minute })} fits />
          </HStack>
        </Field>
      ) : null}
      <Field label="Time zone" detail="The schedule follows this zone, including daylight saving.">
        <input className="routine-input" value={schedule.zone} placeholder="Asia/Taipei" spellCheck={false} autoCorrect="off" onChange={(e) => update({ zone: e.target.value })} />
      </Field>
      <SchedulePreview preview={preview} error={error} />
    </VStack>
  )
}

/** Every N units, for the `interval` kind. */
export function IntervalFields({ schedule, update, preview, error }: { schedule: RoutineScheduleDraft; update: Change; preview: RoutineSchedulePreview | null; error: string | null }) {
  return (
    <VStack gap={14}>
      <Field label="Every">
        <HStack gap={8}>
          <input className="routine-input" inputMode="numeric" value={schedule.amount} placeholder="1" onChange={(e) => update({ amount: e.target.value.replace(/\D/g, '').slice(0, 6) })} />
          <ChoicePicker selection={schedule.unit} options={intervalUnits} onChange={(unit) => update({ unit })} fits />
        </HStack>
      </Field>
      <SchedulePreview preview={preview} error={error} />
    </VStack>
  )
}

/** One quiet line under the fields: what the host read, when it runs next, or why it can't. */
function SchedulePreview({ preview, error }: { preview: RoutineSchedulePreview | null; error: string | null }) {
  return (
    <div className="routine-cron-preview">
      {error ? (
        <Text size="caption" tone="danger" className="routine-cron-error">{error}</Text>
      ) : preview ? (
        <VStack gap={2}>
          <Text size="caption" weight="medium">{preview.summary}</Text>
          {preview.nextRunAt ? (
            <Text size="caption" tone="secondary">Next {new Date(preview.nextRunAt).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}</Text>
          ) : null}
          {preview.warning ? <Text size="caption" tone="secondary">{preview.warning}</Text> : null}
        </VStack>
      ) : (
        <Text size="caption" tone="tertiary">Checking this schedule…</Text>
      )}
    </div>
  )
}
