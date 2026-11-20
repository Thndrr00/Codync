import { useState } from 'react'
import { Button, CardSection, ValueRow } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import type { Hello } from '../../../shared/models'

type Provider = Hello['backends'][number] & { updateAvailable?: boolean }

/**
 * Agent providers on this computer. An "Update available" mark comes from the host's registry
 * check; Update all updates the CLIs Codync downloaded itself and leaves a CLI the person
 * installed, a custom path and a provider no bot runs alone. There is no joined-server part:
 * providers belong to this computer only.
 */
export function ProvidersSection() {
  const tr = useTr()
  const app = useApp()
  const local = useModel(app.local)
  const [busy, setBusy] = useState(false)
  const providers: Provider[] = local?.hello?.backends ?? []
  const pending = providers.filter((provider) => provider.updateAvailable === true)
  const updateAll = async () => {
    if (!local || busy) return
    setBusy(true)
    try {
      const client = await local.ready()
      await client.call('updateAllProviders', {})
      await local.refreshBackends()
    } finally {
      setBusy(false)
    }
  }
  if (providers.length === 0) return null
  return (
    <CardSection
      title={tr('Providers')}
      footer={tr('Agent CLIs on this computer. Codync updates the ones it downloaded itself, once an hour while your bots are idle.')}
    >
      {pending.length > 0 ? (
        <HStack gap={8}>
          <Text size="caption" tone="secondary">{tr('{n} updates available').replace('{n}', String(pending.length))}</Text>
          <Button kind="secondary" disabled={busy} onClick={() => void updateAll()}>
            {busy ? tr('Updating…') : tr('Update all')}
          </Button>
        </HStack>
      ) : null}
      {providers.map((provider) => (
        <ValueRow key={provider.id} label={provider.name}>
          {provider.updateAvailable ? <Text size="caption" tone="attention">{tr('Update available')}</Text> : null}
        </ValueRow>
      ))}
    </CardSection>
  )
}
