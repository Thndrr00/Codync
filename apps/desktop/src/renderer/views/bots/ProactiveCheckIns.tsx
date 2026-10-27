import { DEFAULT_PROACTIVE_SETTINGS, type ProactiveSettings } from '@shared/bdots-models'
import { CardSection, ChoicePicker, ValueRow } from '../../components/Controls'
import { SwitchRow } from './parts'

const INTERVALS = [1, 2, 4, 8, 12, 24]
const DAILY_LIMITS = [1, 2, 3, 5, 8]
const HOURS = Array.from({ length: 24 }, (_, hour) => ({ id: String(hour), label: `${String(hour).padStart(2, '0')}:00` }))

/** How often the bot may wake on its own, when it stays quiet and how many times a day (the host runs them). */
export function ProactiveCheckIns({ settings, onChange }: { settings?: ProactiveSettings; onChange: (next: ProactiveSettings) => void }) {
  const current = settings ?? DEFAULT_PROACTIVE_SETTINGS
  const update = (patch: Partial<ProactiveSettings>) => onChange({ ...current, ...patch })
  return (
    <CardSection
      title="Proactive check-ins"
      footer="The bot wakes up on its own to review what is open and only writes to you when it has something to say."
    >
      <SwitchRow title="Proactive check-ins" detail="Let this bot check in without being asked" on={current.enabled} onChange={(enabled) => update({ enabled })} />
      {current.enabled ? (
        <>
          <ValueRow label="Check in every">
            <ChoicePicker
              selection={String(current.everyHours)}
              options={INTERVALS.map((hours) => ({ id: String(hours), label: hours === 1 ? '1 hour' : `${hours} hours` }))}
              onChange={(id) => update({ everyHours: Number(id) })}
            />
          </ValueRow>
          <ValueRow label="Quiet from">
            <ChoicePicker selection={String(current.quietStart)} options={HOURS} onChange={(id) => update({ quietStart: Number(id) })} />
          </ValueRow>
          <ValueRow label="Quiet until">
            <ChoicePicker selection={String(current.quietEnd)} options={HOURS} onChange={(id) => update({ quietEnd: Number(id) })} />
          </ValueRow>
          <ValueRow label="At most per day">
            <ChoicePicker
              selection={String(current.maxPerDay)}
              options={DAILY_LIMITS.map((count) => ({ id: String(count), label: String(count) }))}
              onChange={(id) => update({ maxPerDay: Number(id) })}
            />
          </ValueRow>
        </>
      ) : null}
    </CardSection>
  )
}
