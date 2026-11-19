import { useEffect, useState } from 'react'
import { CardSection, ChoicePicker, ValueRow } from '../../components/Controls'
import { Text } from '../../components/Text'
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
    if (busy || limit === apps.limit) return
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
        <Text tone="secondary" tabular>
          {tr('{used} of {limit}').replace('{used}', String(apps.count)).replace('{limit}', String(apps.limit))}
        </Text>
      </ValueRow>
      <ValueRow label="Bots that made apps">
        <Text tone="secondary" tabular>{String(apps.botsWithApps)}</Text>
      </ValueRow>
      <ValueRow label="Local app limit">
        <ChoicePicker
          selection={apps.limit}
          options={apps.choices.map((choice) => ({ id: choice, label: String(choice) }))}
          onChange={(limit) => void save(limit)}
        />
      </ValueRow>
      {error ? (
        <Text size="caption" tone="danger" aria-live="polite">{error}</Text>
      ) : busy ? (
        <Text size="caption" tone="secondary" aria-live="polite">{tr('Saving…')}</Text>
      ) : null}
    </CardSection>
  )
}
