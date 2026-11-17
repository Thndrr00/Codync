import { useEffect, useState } from 'react'
import { Button, CardForm, CardSection, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { fetchMarketplaceMeta, fetchReputation, rateApp, recordInstall, type AppReputation } from '../../lib/reputation'
import { errorText, type MarketApp } from './market-models'
import { ErrorText, Field, ItemRow, MarketSection, ServiceLogo, WebLink } from './pieces'
import { usePlugins } from './plugins'
import { useTr } from '../../lib/use-language'

/**
 * An app's page: what it does, how to set it up (steps, a key, or its own sign-in), and example
 * prompts. Add connects it and starts the sign-in when it wants one.
 */
export function AppSheet({ app, added, onAdded, onError }: { app: MarketApp; added: boolean; onAdded: () => void; onError: (message: string) => void }) {
  const tr = useTr()
  const store = useStore()
  const plugins = usePlugins(store)
  const [values, setValues] = useState<Record<string, string>>({})
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [reputation, setReputation] = useState<AppReputation | null>(null)
  const [changelog, setChangelog] = useState<{ version: string; notes: string } | null>(null)
  const [verified, setVerified] = useState(false)
  const [rating, setRating] = useState(false)
  const installed = added || app.installed
  const missing = app.fields.some((f) => (f.required || f.url) && !(values[f.id] ?? '').trim())

  useEffect(() => {
    let alive = true
    void fetchReputation().then((all) => {
      if (alive) setReputation(all.get(app.id) ?? { appId: app.id, stars: 0, count: 0, installs: 0 })
    })
    void fetchMarketplaceMeta().then((meta) => {
      if (!alive) return
      setChangelog(meta.changelog.get(app.id) ?? null)
      setVerified(meta.publishers.has(app.creator))
    })
    return () => {
      alive = false
    }
  }, [app.id, app.creator])

  const rate = async (stars: number) => {
    setRating(true)
    try {
      await rateApp(app.id, stars)
      const all = await fetchReputation()
      setReputation(all.get(app.id) ?? { appId: app.id, stars, count: reputation?.count ?? 1, installs: reputation?.installs ?? 0 })
      setError(null)
    } catch (e) {
      setError(errorText(e))
    }
    setRating(false)
  }

  const add = async () => {
    setBusy(true)
    setError(null)
    try {
      await plugins.installApp(app, values)
      void recordInstall(app.id)
      onAdded()
    } catch (e) {
      setError(errorText(e))
      onError(errorText(e))
    }
    setBusy(false)
  }

  const trailing = installed ? (
    <span title={tr('Added')} aria-label={tr('Added')} className="skill-added">
      <Icon name="checkmark" size={13} weight="semibold" />
    </span>
  ) : busy ? <Spinner /> : (
    <Button disabled={missing} onClick={() => void add()}>
      <Icon name="plus" size={12} />{app.setup === 'link' ? tr('Connect') : tr('Add app')}
    </Button>
  )

  return (
    <SheetPage>
      <ModalHeader icon={<ServiceLogo website={app.website} name={app.name} size={28} />} title={app.name} trailing={trailing} />
      <CardForm>
        <CardSection>
          <VStack gap={6}>
            <Text tone="text" weight="medium">{app.tagline}</Text>
            <Text tone="secondary">{app.description}</Text>
            {app.website ? <WebLink title={app.website} url={app.website} className="small truncate" /> : null}
            <dl className="market-facts">
              <dt>{tr('Creator')}</dt>
              <dd>
                {app.creator}
                {verified ? <> · <span title={tr('Verified by Codync')}>✓ {tr('Verified')}</span></> : null}
              </dd>
              <dt>{tr('Origin')}</dt><dd>{app.origin ? <WebLink title={app.creator} url={app.origin} className="small" /> : tr('Reviewed by CopilotKit')}</dd>
              <dt>{tr('Category')}</dt><dd>{tr(app.category)}</dd>
              <dt>{tr('Version')}</dt><dd>{app.version}</dd>
              <dt>{tr('Updated')}</dt><dd>{app.updated}</dd>
              <dt>{tr('Reputation')}</dt>
              <dd>
                <HStack gap={6} align="center">
                  <span className="market-stars" aria-label={tr('{count} ratings').replace('{count}', String(reputation?.count ?? 0))}>
                    {[1, 2, 3, 4, 5].map((star) => (
                      <Icon key={star} name={star <= Math.round(reputation?.stars ?? 0) ? 'star.fill' : 'star'} size={12} />
                    ))}
                  </span>
                  <Text size="caption" tone="secondary">
                    {reputation && reputation.count ? `${reputation.stars.toFixed(1)} · ${reputation.count} · ${reputation.installs} installs` : tr('Not rated yet')}
                  </Text>
                </HStack>
              </dd>
            </dl>
            <HStack gap={6} align="center">
              <Text size="caption" tone="secondary">{tr('Rate it')}</Text>
              {[1, 2, 3, 4, 5].map((star) => (
                <Button key={star} kind="secondary" disabled={rating} onClick={() => void rate(star)}>
                  <Icon name="star" size={12} />
                  {star}
                </Button>
              ))}
            </HStack>
          </VStack>
        </CardSection>
        {changelog ? (
          <CardSection title={tr("What's new")} footer={`${tr('Version')} ${changelog.version}`}>
            <Text tone="secondary">{changelog.notes}</Text>
          </CardSection>
        ) : null}
        {app.setup === 'local' && app.steps.length ? (
          <CardSection title={tr('Setup')} footer={tr('Runs on {name}.').replaceAll('{name}', store.hostName)}>
            <ol className="market-steps">
              {app.steps.map((step) => <li key={step}>{step}</li>)}
            </ol>
            {app.docsUrl ? <WebLink title={app.docsLabel ?? tr('Open the guide')} url={app.docsUrl} className="small" /> : null}
          </CardSection>
        ) : null}
        {app.setup === 'key' ? (
          <CardSection title={tr('Setup')} footer={tr('Keys are saved on {name} only.').replaceAll('{name}', store.hostName)}>
            {app.fields.map((field) => (
              <VStack key={field.id} gap={4}>
                <Field
                  label={field.label}
                  value={values[field.id] ?? ''}
                  secret={!field.url}
                  placeholder={field.placeholder ?? field.label}
                  onChange={(v) => setValues((old) => ({ ...old, [field.id]: v }))}
                />
                {field.hint ? <Text size="caption" tone="secondary">{field.hint}</Text> : null}
              </VStack>
            ))}
            {app.docsUrl ? <WebLink title={app.docsLabel ?? tr('Get a key')} url={app.docsUrl} className="small" /> : null}
          </CardSection>
        ) : null}
        {app.setup === 'link' || app.setup === 'open' ? (
          <CardSection title={tr('Setup')} footer={tr('Keys and sign-ins stay on {name}.').replaceAll('{name}', store.hostName)}>
            <Text tone="secondary">
              {app.setup === 'link' ? tr('{name} asks you to sign in to your own account in the browser.').replaceAll('{name}', app.name) : tr('Nothing to fill in here.')}
            </Text>
            {app.fields.map((field) => (
              <VStack key={field.id} gap={4}>
                <Field
                  label={field.label}
                  value={values[field.id] ?? ''}
                  secret={!field.url}
                  placeholder={field.placeholder ?? field.label}
                  onChange={(v) => setValues((old) => ({ ...old, [field.id]: v }))}
                />
                {field.hint ? <Text size="caption" tone="secondary">{field.hint}</Text> : null}
              </VStack>
            ))}
            {app.fields.length && app.docsUrl ? <WebLink title={app.docsLabel ?? tr('Open the guide')} url={app.docsUrl} className="small" /> : null}
          </CardSection>
        ) : null}
        {app.prompts.length ? (
          <MarketSection title={tr('Try it')}>
            {app.prompts.map((p) => (
              <ItemRow
                key={p.id}
                title={p.text}
                subtitle={tr('Click to copy')}
                icon={<div className="market-tile"><Icon name="text.bubble" size={14} /></div>}
                onOpen={() => void navigator.clipboard.writeText(p.text)}
              />
            ))}
          </MarketSection>
        ) : null}
        {error ? <CardSection><ErrorText text={error} /></CardSection> : null}
      </CardForm>
    </SheetPage>
  )
}
