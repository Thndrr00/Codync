import { useState } from 'react'
import { ChoicePicker } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { TimeZonePicker } from '../routines/TimeZonePicker'
import { errorText } from '../routines/routine'
import { dayLabel } from './schedule'

export interface ScheduleBot {
  id: string
  name: string
}

export interface ScheduleSlot {
  day: string
  /** An hour clicked in the day view; null when a whole day was chosen. */
  hour: number | null
}

/**
 * A shared Dialog that creates a one-off routine at the clicked day and hour. The date and time,
 * the time zone and the bot are already set; only the name and instructions are left to type.
 */
export function ScheduleCreateDialog({ slot, zone, onZone, bots, botId, onClose, onCreated }: {
  slot: ScheduleSlot
  zone: string
  onZone: (zone: string) => void
  bots: ScheduleBot[]
  botId: string
  onClose: () => void
  onCreated: (at: number) => void
}) {
  const tr = useTr()
  const store = useStore()
  const [bot, setBot] = useState(bots.some((candidate) => candidate.id === botId) ? botId : (bots[0]?.id ?? botId))
  const [name, setName] = useState('')
  const [instruction, setInstruction] = useState('')
  const [minute, setMinute] = useState('00')
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const hour = slot.hour ?? 9
  const timeLabel = `${String(hour).padStart(2, '0')}:${minute.padStart(2, '0')}`

  const submit = async () => {
    if (saving || !bot || !name.trim() || !instruction.trim()) return
    setSaving(true)
    setError(null)
    try {
      const result = await (await store.ready()).call<{ at: number }>('scheduleCreateAt', {
        botId: bot,
        name: name.trim(),
        instruction: instruction.trim(),
        date: slot.day,
        hour,
        minute: Number(minute),
        timeZone: zone,
      })
      onCreated(result.at)
    } catch (cause) {
      setError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open
      title={tr('New routine')}
      message={`${dayLabel(slot.day)} · ${timeLabel} · ${zone.replaceAll('_', ' ')}`}
      actions={[{ title: saving ? tr('Creating…') : tr('Create routine'), disabled: saving || !name.trim() || !instruction.trim(), action: () => void submit() }]}
      onCancel={onClose}
      onClose={() => {}}
    >
      <div className="schedule-create">
        <input
          className="dialog-field"
          data-autofocus
          value={name}
          aria-label={tr('Routine name')}
          placeholder={tr('Routine name')}
          onChange={(event) => setName(event.target.value)}
        />
        <textarea
          className="dialog-field schedule-instructions"
          value={instruction}
          aria-label={tr('Instructions')}
          placeholder={tr('What should this routine do?')}
          onChange={(event) => setInstruction(event.target.value)}
        />
        <div className="schedule-create-row">
          <Text size="caption" tone="secondary">{tr('Time zone')}</Text>
          <TimeZonePicker zone={zone} onChange={onZone} />
        </div>
        <div className="schedule-create-row">
          <Text size="caption" tone="secondary">{tr('Minute')}</Text>
          <input
            className="schedule-input schedule-minute"
            value={minute}
            inputMode="numeric"
            aria-label={tr('Minute')}
            onChange={(event) => {
              const typed = event.target.value.replace(/\D/g, '').slice(0, 2)
              if (typed === '' || Number(typed) < 60) setMinute(typed)
            }}
          />
        </div>
        {bots.length > 1 ? (
          <div className="schedule-create-row">
            <Text size="caption" tone="secondary">{tr('Bot')}</Text>
            <ChoicePicker selection={bot} options={bots.map((candidate) => ({ id: candidate.id, label: candidate.name }))} onChange={setBot} fill="var(--surface)" />
          </div>
        ) : null}
        {error ? <Text size="caption" tone="danger">{error}</Text> : null}
      </div>
    </Dialog>
  )
}
