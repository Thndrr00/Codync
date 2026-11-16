import { useEffect, useState } from 'react'
import { CardSection, SegmentedChoice, ValueRow } from '../../components/Controls'
import { Text } from '../../components/Text'
import { fill } from '../../lib/fill.ts'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import { errorText } from './parts'

/**
 * The computer-wide ceiling on local mini-apps: how many the bots on this computer may keep between
 * them. It is a safeguard here, not a plan; only a new app takes a slot, and a bot may always update
 * an app it already made.
 */
export function LocalAppsSection() {
  const app = useApp()
  const local = useModel(app.local)
  const tr = useTr()
  const apps = local?.localApps ?? null
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    void local?.refreshLocalApps()
  }, [local])
  if (!apps) return null
  const save = async (limit: number) => {
    setError(null)
    setBusy(true)
    try {
      await local?.setLocalAppsLimit(limit)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }
  return (
    <CardSection
      title="Local apps"
      footer="A local safeguard on this computer, not a plan: the bots here may keep up to this many mini-apps between them. Changing an app a bot already made never counts, and deleting one makes room."
    >
      <ValueRow label="Apps on this computer">
        <Text tone="secondary">{fill(tr('{used} of {limit}'), { used: String(apps.count), limit: String(apps.limit) })}</Text>
      </ValueRow>
      <ValueRow label="Local app limit">
        <SegmentedChoice
          selection={String(apps.limit)}
          options={apps.choices.map((choice) => ({ id: String(choice), label: String(choice) }))}
          onChange={(id) => void save(Number(id))}
        />
      </ValueRow>
      {error ? <Text size="footnote" tone="danger">{error}</Text> : null}
      {busy ? <Text size="footnote" tone="secondary">{tr('Saving…')}</Text> : null}
    </CardSection>
  )
}
