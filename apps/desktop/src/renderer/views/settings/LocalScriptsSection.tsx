import { useEffect, useState } from 'react'
import { CardSection, SegmentedChoice, ValueRow } from '../../components/Controls'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { errorText } from './parts'

/** The hourly ceiling on requests that local scripts send to routines on this computer. */
export function LocalScriptsSection() {
  const app = useApp()
  const local = useModel(app.local)
  const limit = local?.scriptLimit ?? null
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    void local?.refreshScriptLimit()
  }, [local])
  if (!limit) return null
  const save = async (perHour: number) => {
    setError(null)
    setBusy(true)
    try {
      await local?.setScriptLimit(perHour)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }
  return (
    <CardSection
      title="Local scripts"
      footer="How many requests a script on this computer may send to routines each hour, for every agent together. Past it the address answers 429 and the script tries again later."
    >
      <ValueRow label="Requests an hour">
        <SegmentedChoice
          selection={String(limit.perHour)}
          options={limit.choices.map((choice) => ({ id: String(choice), label: String(choice) }))}
          onChange={(id) => void save(Number(id))}
        />
      </ValueRow>
      {error ? <Text size="footnote" tone="danger">{error}</Text> : null}
      {busy ? <Text size="footnote" tone="secondary">Saving…</Text> : null}
    </CardSection>
  )
}
