import { useEffect, useState } from 'react'
import { Button, CardSection, ChoicePicker, SegmentedChoice, ValueRow } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { fill } from '../../lib/fill.ts'
import { useTr } from '../../lib/use-language'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'
import { errorText } from './parts'

/** "10 s", "1 min", "1 min 30 s": how long the plan card waits before it starts the work. */
export function delayText(ms: number) {
  const minutes = Math.floor(ms / 60_000)
  const seconds = Math.round((ms % 60_000) / 1_000)
  if (minutes === 0) return `${seconds} s`
  if (seconds === 0) return `${minutes} min`
  return `${minutes} min ${seconds} s`
}

type Unit = 's' | 'min'
const UNIT_MS: Record<Unit, number> = { s: 1_000, min: 60_000 }

/**
 * "Start the work": when a plan card of a terminal agent starts the plan by itself, for every agent on this
 * computer. Ported from OpenBot's Settings > Agents > "Start the work".
 */
export function PlanStartSection() {
  const app = useApp()
  const local = useModel(app.local)
  const tr = useTr()
  const start = local?.planStart ?? null
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [customOpen, setCustomOpen] = useState(false)
  const [amount, setAmount] = useState('1')
  const [unit, setUnit] = useState<Unit>('min')
  useEffect(() => {
    void local?.refreshPlanStart()
  }, [local])
  if (!start) return null
  const afterMs = start.afterMs
  const isPreset = afterMs !== null && start.presets.includes(afterMs)
  const options = [
    { id: 'wait', label: tr('When I press Start') },
    ...start.presets.map((ms) => ({ id: String(ms), label: fill(tr('After {time}'), { time: delayText(ms) }) })),
    ...(afterMs !== null && !isPreset
      ? [{ id: String(afterMs), label: fill(tr('After {time}'), { time: delayText(afterMs) }) }]
      : []),
    { id: 'custom', label: tr('Custom time…') },
  ]
  const save = async (next: number | null) => {
    setError(null)
    setBusy(true)
    try {
      await local?.setPlanStart(next)
      setCustomOpen(false)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }
  const typedMs = () => {
    if (!/^\d+$/.test(amount.trim())) return null
    const ms = Number(amount) * UNIT_MS[unit]
    return ms >= start.min && ms <= start.max ? ms : null
  }
  const custom = typedMs()
  return (
    <CardSection
      title="Plan cards"
      footer={
        afterMs === null
          ? tr('The plan waits until you press Start. You get a notification.')
          : fill(tr('The work starts after {time}, or when you press Start.'), { time: delayText(afterMs) })
      }
    >
      <ValueRow label="Start the work">
        <ChoicePicker
          selection={afterMs === null ? 'wait' : String(afterMs)}
          options={options}
          onChange={(id) => {
            if (id === 'custom') {
              setAmount(afterMs === null ? '1' : String(Math.max(1, Math.round(afterMs / 60_000))))
              setUnit('min')
              setCustomOpen(true)
              return
            }
            void save(id === 'wait' ? null : Number(id))
          }}
        />
      </ValueRow>
      {customOpen ? (
        <>
          <ValueRow label="Time before the work starts">
            <HStack gap={6}>
              <Field value={amount} onChange={setAmount} placeholder="5" />
              <SegmentedChoice
                selection={unit}
                options={[{ id: 's', label: 'sec' }, { id: 'min', label: 'min' }]}
                onChange={(next) => setUnit(next as Unit)}
              />
            </HStack>
          </ValueRow>
          <HStack gap={8}>
            <Text size="footnote" tone={custom === null ? 'danger' : 'secondary'}>
              {custom === null ? tr('Enter a whole number of at least 5 seconds.') : tr('At least 5 seconds.')}
            </Text>
            <Button disabled={custom === null || busy} onClick={() => custom !== null && void save(custom)}>
              {tr('Set')}
            </Button>
            <Button kind="secondary" onClick={() => setCustomOpen(false)}>{tr('Cancel')}</Button>
          </HStack>
        </>
      ) : null}
      {error ? <Text size="footnote" tone="danger">{error}</Text> : null}
      {busy ? <Text size="footnote" tone="secondary">{tr('Saving…')}</Text> : null}
    </CardSection>
  )
}
