import { useCallback, useEffect, useState } from 'react'
import {
  componentLabel,
  type CatalogueComponent,
  type ComponentCard,
  type ComponentGallery,
  type ComponentInstance,
} from '@shared/components.ts'
import { Button, IconButton, SegmentedChoice, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import type { BotStore } from '../../store/bot-store'
import { useTr } from '../../lib/use-language'
import { ComponentPreview } from '../thread/ComponentCard'
import { ComponentPlayground } from './ComponentPlayground'
import './components.css'

/**
 * A computer's component gallery: the cards Codync can draw (`metric`, `table`, …) with a live preview,
 * which bots may use each one, and the playground that authors a card from a component's schema. The
 * host owns the catalogue and checks every card; this page only reads it and sends back choices.
 */
export function ComponentsView({ store }: { store: BotStore }) {
  const tr = useTr()
  const [gallery, setGallery] = useState<ComponentGallery | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [tab, setTab] = useState<'gallery' | 'playground'>('gallery')
  const [selected, setSelected] = useState<string | null>(null)
  const [instance, setInstance] = useState<ComponentInstance | null>(null)
  const [live, setLive] = useState<ComponentCard | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      const client = await store.ready()
      setGallery(await client.call<ComponentGallery>('listComponents', {}))
      setError(null)
    } catch (problem) {
      setError(problem instanceof Error ? problem.message : String(problem))
    }
  }, [store])

  useEffect(() => {
    void load()
  }, [load])

  const chosen = gallery?.components.find((component) => component.name === selected) ?? gallery?.components[0]
  const card = live ?? sampleCard(chosen, instance)

  /** Runs one change on the host, then re-reads the gallery. */
  const change = async (method: string, body: Record<string, unknown>) => {
    setBusy(true)
    try {
      const client = await store.ready()
      await client.call(method, body)
      setError(null)
      await load()
    } catch (problem) {
      setError(problem instanceof Error ? problem.message : String(problem))
    } finally {
      setBusy(false)
    }
  }

  /** A published card with data functions shows this computer's own state, not the sample. */
  const previewInstance = async (saved: ComponentInstance) => {
    setInstance(saved)
    setSelected(saved.name)
    setLive(null)
    if (!saved.data?.length) return
    try {
      const client = await store.ready()
      const answer = await client.call<{ component: ComponentCard }>('previewComponent', {
        name: saved.name,
        title: saved.title || null,
        props: saved.props,
        data: saved.data,
      })
      setLive(answer.component)
    } catch (problem) {
      setError(problem instanceof Error ? problem.message : String(problem))
    }
  }

  if (!gallery) {
    return (
      <VStack gap={8} className="components-view">
        {error ? <Text size="callout" tone="attention">{error}</Text> : <Text size="callout" tone="secondary">{tr('Loading…')}</Text>}
      </VStack>
    )
  }

  return (
    <VStack gap={12} className="components-view">
      <HStack gap={10}>
        <SegmentedChoice
          selection={tab}
          options={[{ id: 'gallery', label: tr('Gallery') }, { id: 'playground', label: tr('Playground') }]}
          onChange={(next) => {
            setTab(next)
            setLive(null)
          }}
        />
        <Spacer />
      </HStack>
      {error ? <Text size="caption" tone="attention" className="declarative-text">{error}</Text> : null}
      {tab === 'playground' ? (
        <ComponentPlayground
          store={store}
          components={gallery.components}
          instance={instance}
          onSaved={() => {
            setInstance(null)
            setLive(null)
            void load()
          }}
        />
      ) : (
        <HStack gap={14} align="flex-start" className="components-gallery">
          <VStack gap={6} className="components-list">
            {gallery.components.map((component) => (
              <button
                key={component.name}
                type="button"
                className={`components-row ${chosen?.name === component.name && !instance ? 'current' : ''}`}
                onClick={() => {
                  setSelected(component.name)
                  setInstance(null)
                  setLive(null)
                }}
              >
                <VStack gap={2} grow>
                  <Text size="callout" truncate>{tr(componentLabel(component.name))}</Text>
                  <Text size="caption2" tone="tertiary" truncate>{tr(component.summary)}</Text>
                </VStack>
                {component.available === false ? <Icon name="eye.slash" size={12} color="var(--secondary)" /> : null}
              </button>
            ))}
            {gallery.instances.length ? <SavedCards gallery={gallery} instance={instance} onPick={previewInstance} onPublish={change} onDelete={change} busy={busy} /> : null}
          </VStack>
          <VStack gap={12} grow className="components-detail">
            {card ? (
              <div className="component-preview">
                <ComponentPreview card={card} />
              </div>
            ) : null}
            {chosen ? <CatalogueNotes component={chosen} /> : null}
            {instance ? (
              <HStack gap={8} wrap>
                <Button
                  kind="secondary"
                  disabled={busy}
                  onClick={() => void change('publishComponent', { id: instance.id, published: !instance.published })}
                >
                  {instance.published ? tr('Take out of the gallery') : tr('Show in the gallery')}
                </Button>
                <Button kind="secondary" disabled={busy} onClick={() => void change('deleteComponent', { id: instance.id })}>
                  {tr('Delete card')}
                </Button>
                <Text size="caption2" tone="tertiary">
                  {instance.published ? tr('Every bot can show it') : tr('Only the playground has it')}
                </Text>
              </HStack>
            ) : chosen ? (
              <Bots allowed={chosen} gallery={gallery} busy={busy} onChange={(bot, published) => void change('setComponentAccess', { bot, component: chosen.name, published })} />
            ) : null}
          </VStack>
        </HStack>
      )}
    </VStack>
  )
}

/** The card a catalogue component's own sample makes, or a saved card's props when one is picked. */
function sampleCard(component: CatalogueComponent | undefined, instance: ComponentInstance | null): ComponentCard | null {
  if (instance) return { version: 1, name: instance.name, title: instance.title || undefined, props: instance.props }
  if (!component) return null
  return { version: 1, name: component.name, title: undefined, props: component.sample }
}

function CatalogueNotes({ component }: { component: CatalogueComponent }) {
  const tr = useTr()
  return (
    <VStack gap={4}>
      <Text size="callout" tone="text">{tr(component.summary)}</Text>
      <HStack gap={6} wrap>
        <Text size="caption2" tone="tertiary">{tr('Props')}:</Text>
        <Text size="caption2" tone="secondary">{Object.keys(component.schema.properties).join(', ')}</Text>
      </HStack>
      <HStack gap={6} wrap>
        <Text size="caption2" tone="tertiary">{tr('Data functions')}:</Text>
        <Text size="caption2" tone="secondary">{component.data.length ? component.data.join(', ') : tr('None')}</Text>
      </HStack>
    </VStack>
  )
}

/** Which bots may use this component: a switch each, off for a withheld bot. */
function Bots({ allowed, gallery, busy, onChange }: {
  allowed: CatalogueComponent
  gallery: ComponentGallery
  busy: boolean
  onChange: (bot: string, published: boolean) => void
}) {
  const tr = useTr()
  if (!gallery.bots.length) return null
  return (
    <VStack gap={6}>
      <Text size="caption" tone="secondary" weight="medium">{tr('Who may use it')}</Text>
      {gallery.bots.map((bot) => {
        const withheld = bot.withheld.includes(allowed.name)
        return (
          <HStack key={bot.id} gap={8}>
            <Icon name={bot.kind === 'group' ? 'person.2' : 'bubble.left'} size={12} color="var(--secondary)" />
            <Text size="callout" grow truncate>{bot.name}</Text>
            <ByteSwitch
              on={!withheld}
              disabled={busy}
              label={withheld ? tr('Withheld') : tr('Allowed')}
              onChange={(on) => onChange(bot.id, on)}
            />
          </HStack>
        )
      })}
    </VStack>
  )
}

function ByteSwitch({ on, disabled, label, onChange }: { on: boolean; disabled: boolean; label: string; onChange: (on: boolean) => void }) {
  return (
    <HStack gap={6}>
      <Text size="caption2" tone="tertiary">{label}</Text>
      <Switch on={on} onChange={onChange} disabled={disabled} label={label} />
    </HStack>
  )
}

/** The playground's saved cards: each opens its preview, publishes or leaves the gallery. */
function SavedCards({ gallery, instance, onPick, onPublish, onDelete, busy }: {
  gallery: ComponentGallery
  instance: ComponentInstance | null
  onPick: (instance: ComponentInstance) => void
  onPublish: (method: string, body: Record<string, unknown>) => Promise<void>
  onDelete: (method: string, body: Record<string, unknown>) => Promise<void>
  busy: boolean
}) {
  const tr = useTr()
  return (
    <VStack gap={4}>
      <Text size="caption" tone="tertiary" weight="medium">{tr('Saved cards')}</Text>
      {gallery.instances.map((saved) => (
        <HStack key={saved.id} gap={6} className={`components-row ${instance?.id === saved.id ? 'current' : ''}`}>
          <button type="button" className="components-row-main" onClick={() => onPick(saved)}>
            <VStack gap={2} grow>
              <Text size="callout" truncate>{saved.title || tr(componentLabel(saved.name))}</Text>
              <Text size="caption2" tone="tertiary" truncate>
                {saved.published ? tr('In the gallery') : tr('Draft')}
                {saved.valid === false ? ` · ${tr('Needs updating')}` : ''}
              </Text>
            </VStack>
          </button>
          <IconButton
            title={saved.published ? tr('Take out of the gallery') : tr('Show in the gallery')}
            icon={saved.published ? 'eye.slash' : 'eye'}
            disabled={busy}
            onClick={() => void onPublish('publishComponent', { id: saved.id, published: !saved.published })}
          />
          <IconButton title={tr('Delete card')} icon="trash" disabled={busy} onClick={() => void onDelete('deleteComponent', { id: saved.id })} />
        </HStack>
      ))}
    </VStack>
  )
}
