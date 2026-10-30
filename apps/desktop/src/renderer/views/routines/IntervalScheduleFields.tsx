import { CardSection, ChoicePicker } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { intervalUnits } from '../../lib/routine-schedule'
import { LabeledRow, SchedulePreview } from './RoutineScheduleFields'
import type { RoutineScheduleDraft, RoutineSchedulePreview } from './routine'

export function IntervalScheduleFields({ schedule, update, preview, error }: {
  schedule: RoutineScheduleDraft
  update: (change: Partial<RoutineScheduleDraft>) => void
  preview: RoutineSchedulePreview | null
  error: string | null
}) {
  return <CardSection footer={<SchedulePreview preview={preview} error={error} />}>
    <LabeledRow label="Every">
      <HStack gap={8}>
        <input className="schedule-input" aria-label="Interval amount" inputMode="numeric" value={schedule.amount}
          onChange={(event) => update({ amount: event.target.value.replace(/\D/g, '').slice(0, 6) })} />
        <ChoicePicker selection={schedule.unit} options={intervalUnits} onChange={(unit) => update({ unit })} fits />
      </HStack>
    </LabeledRow>
  </CardSection>
}
