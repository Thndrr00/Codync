import { useState } from 'react'
import { Button, CardSection, Reveal } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { demoMode } from '../../store/demo-mode'

/** Settings → Try demo mode: sample bots on an in-memory host, to look around without touching anything real. */
export function DemoSection() {
  const demo = useModel(demoMode)
  const tr = useTr()
  const [busy, setBusy] = useState(false)
  const [loaded, setLoaded] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = async () => {
    const model = demoMode.model
    if (!model) return
    setBusy(true)
    setError(null)
    setLoaded(false)
    try {
      const client = await model.local?.ready()
      if (!client) throw new Error(tr('The demo is not ready yet.'))
      await client.call('loadSampleData', {})
      setLoaded(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <CardSection title="Demo mode" footer="Look around with sample bots, a session, an approval and a routine. Nothing is saved or sent, and leaving brings back exactly what you had.">
      {demo.active ? (
        <VStack gap={8}>
          <HStack gap={8}>
            <Button kind="secondary" onClick={() => void load()} disabled={busy}>{busy ? tr('Loading…') : tr('Load sample data')}</Button>
            <Button kind="secondary" onClick={() => demoMode.exit()}>Exit demo mode</Button>
          </HStack>
          <Reveal show={loaded}>
            {loaded ? <Text size="caption" tone="secondary">{tr('Sample data loaded: bots, messages, a thread, artifacts and library folders.')}</Text> : null}
          </Reveal>
          {error ? <Text size="caption" tone="danger" role="alert">{error}</Text> : null}
        </VStack>
      ) : (
        <Button kind="secondary" onClick={() => demoMode.enter()}>Try demo mode</Button>
      )}
    </CardSection>
  )
}
