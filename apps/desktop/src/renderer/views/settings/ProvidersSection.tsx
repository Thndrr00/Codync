import { useState } from 'react'
import { Button, CardSection, ValueRow } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import type { Hello, Usage } from '../../../shared/models'

type Provider = Hello['backends'][number] & {
  updateAvailable?: boolean
  /** Fixed host text when this computer's processor cannot run the provider, never a download error. */
  capability?: string | null
}

/** The host's credit state on the usage event; additive, so an older host simply sends none. */
type CreditUsage = Usage & {
  creditOffers?: { id: string; credits: number; resetsAt?: number | null }[]
  usingCredits?: { id: string; until: number; creditsLeft: number }[]
}

/**
 * Agent providers on this computer. An "Update available" mark comes from the host's registry
 * check; Update all updates the CLIs Codync downloaded itself and leaves a CLI the person
 * installed, a custom path and a provider no bot runs alone. A provider whose spent window has
 * extra credits offers Reset usage, which spends one credit and ends the limit hold on the host.
 * There is no joined-server part: providers belong to this computer only.
 */
export function ProvidersSection() {
  const tr = useTr()
  const app = useApp()
  const local = useModel(app.local)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const providers: Provider[] = local?.hello?.backends ?? []
  const pending = providers.filter((provider) => provider.updateAvailable === true)
  const usage = local?.usage as CreditUsage | undefined
  const baseId = (id: string) => id.split(':')[0]
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
  const reset = async (id: string) => {
    if (!local || busy) return
    setBusy(true)
    setError(null)
    try {
      await (await local.ready()).call('resetUsageCredits', { id })
      await local.refreshUsage()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
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
      {providers.map((provider) => {
        const offer = usage?.creditOffers?.find((item) => baseId(item.id) === provider.id)
        const using = usage?.usingCredits?.find((item) => baseId(item.id) === provider.id)
        return (
          <ValueRow key={provider.id} label={provider.name}>
            {offer ? (
              <Button kind="secondary" disabled={busy} onClick={() => void reset(offer.id)}>
                {tr('Reset usage')}
              </Button>
            ) : null}
            {using ? (
              <Text size="caption" tone="secondary">
                {`${tr('Using credits')} · ${new Date(using.until).toLocaleTimeString()}`}
              </Text>
            ) : null}
            {provider.capability ? <Text size="caption" tone="attention">{tr(provider.capability)}</Text> : null}
            {provider.updateAvailable ? <Text size="caption" tone="attention">{tr('Update available')}</Text> : null}
          </ValueRow>
        )
      })}
      {error ? <Text size="caption" tone="danger">{error}</Text> : null}
    </CardSection>
  )
}
