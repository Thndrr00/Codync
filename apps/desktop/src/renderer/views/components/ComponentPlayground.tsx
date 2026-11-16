import { useEffect, useMemo, useState } from 'react'
import {
  componentLabel,
  propKind,
  propText,
  parsePropText,
  type CatalogueComponent,
  type ComponentCard,
  type ComponentInstance,
} from '@shared/components.ts'
import { Button, ChoicePicker, Spinner, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { ComponentPreview } from '../thread/ComponentCard'
import type { BotStore } from '../../store/bot-store'

/** The playground: pick a catalogue component, write its props in a form from its schema, preview, publish. */
export function ComponentPlayground({ store, components, instance, onSaved }: {
  store: BotStore
  components: CatalogueComponent[]
  instance: ComponentInstance | null
  onSaved: () => void
}) {
  const tr = useTr()
  const [name, setName] = useState(instance?.name ?? components[0]?.name ?? 'metric')
  const spec = components.find((component) => component.name === name) ?? components[0]
  const [title, setTitle] = useState(instance?.title ?? '')
  const [draft, setDraft] = useState<Record<string, string>>({})
  const [functionName, setFunctionName] = useState(instance?.data?.[0]?.function ?? '')
  const [busy, setBusy] = useState<'preview' | 'save' | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [checked, setChecked] = useState(false)
  const [published, setPublished] = useState(instance?.published ?? true)

  // A new component starts from its own sample, so the form is never empty.
  const reset = (component: CatalogueComponent, saved: ComponentInstance | null) => {
    const props = saved?.props ?? component.sample
    const next: Record<string, string> = {}
    for (const key of Object.keys(component.schema.properties)) {
      next[key] = propText((props as Record<string, unknown>)[key])
    }
    setDraft(next)
    setTitle(saved?.title ?? '')
    setPublished(saved?.published ?? true)
    setFunctionName(saved?.data?.[0]?.function ?? '')
  }

  useEffect(() => {
    if (spec) reset(spec, instance?.name === spec.name ? instance : null)
    // A saved card's own values are the starting point; the sample is for a new one.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [spec?.name, instance?.id])

  const props = useMemo(() => {
    const out: Record<string, unknown> = {}
    if (!spec) return out
    for (const [key, schema] of Object.entries(spec.schema.properties)) {
      const text = draft[key] ?? ''
      const kind = propKind(schema)
      if (kind === 'toggle') out[key] = text.trim() === 'true'
      else if (text.trim() !== '') out[key] = parsePropText(kind, text)
    }
    return out
  }, [draft, spec])

  const data = functionName ? [{ function: functionName }] : []
  const preview = useMemo<ComponentCard>(
    () => ({ version: 1, name, title: title.trim() || undefined, props: props as ComponentCard['props'] }),
    [name, title, props],
  )

  const run = async (what: 'preview' | 'save') => {
    setBusy(what)
    setError(null)
    setChecked(false)
    try {
      const client = await store.ready()
      if (what === 'preview') {
        // The host checks and fills the card exactly as a bot's would be: data functions included.
        await client.call('previewComponent', { name, title: title.trim() || null, props, data })
        setChecked(true)
      } else {
        await client.call('saveComponent', {
          id: instance?.name === name ? instance.id : undefined,
          name,
          title,
          props,
          data,
          publish: published,
        })
        onSaved()
      }
    } catch (problem) {
      setError(problem instanceof Error ? problem.message : String(problem))
    } finally {
      setBusy(null)
    }
  }

  if (!spec) {
    return <Text size="callout" tone="secondary">{tr('No components yet.')}</Text>
  }
  return (
    <VStack gap={12} className="component-playground">
      <VStack gap={6}>
        <Text size="caption" tone="secondary" weight="medium">{tr('Component')}</Text>
        <ChoicePicker
          selection={name}
          options={components.map((component) => ({ id: component.name, label: tr(componentLabel(component.name)) }))}
          onChange={(chosen) => {
            setName(chosen)
            const next = components.find((component) => component.name === chosen)
            if (next) reset(next, null)
          }}
        />
      </VStack>
      <label className="component-field">
        <Text size="caption" tone="secondary" weight="medium">{tr('Title')}</Text>
        <input
          className="declarative-input"
          value={title}
          placeholder={tr(componentLabel(spec.name))}
          onChange={(event) => setTitle(event.target.value)}
        />
      </label>
      <VStack gap={10}>
        {Object.entries(spec.schema.properties).map(([key, schema]) => {
          const kind = propKind(schema)
          const required = spec.schema.required.includes(key)
          const label = `${tr(schema.title ?? key)}${required ? ' *' : ''}`
          if (kind === 'choice') {
            return (
              <VStack key={key} gap={6}>
                <Text size="caption" tone="secondary" weight="medium">{label}</Text>
                <ChoicePicker
                  selection={draft[key] ?? ''}
                  options={(schema.enum ?? []).map((choice) => ({ id: choice, label: choice }))}
                  onChange={(choice) => setDraft((current) => ({ ...current, [key]: choice }))}
                />
              </VStack>
            )
          }
          if (kind === 'toggle') {
            return (
              <Switch
                key={key}
                on={(draft[key] ?? '') === 'true'}
                onChange={(on) => setDraft((current) => ({ ...current, [key]: String(on) }))}
              >
                <Text size="callout">{label}</Text>
              </Switch>
            )
          }
          const multiline = kind === 'rows' || kind === 'json' || kind === 'list' || kind === 'numbers'
          return (
            <label key={key} className="component-field">
              <Text size="caption" tone="secondary" weight="medium">{label}</Text>
              {multiline ? (
                <textarea
                  className="declarative-input declarative-textarea"
                  rows={kind === 'rows' ? 4 : 3}
                  value={draft[key] ?? ''}
                  onChange={(event) => setDraft((current) => ({ ...current, [key]: event.target.value }))}
                />
              ) : (
                <input
                  className="declarative-input"
                  inputMode={kind === 'number' ? 'decimal' : undefined}
                  value={draft[key] ?? ''}
                  onChange={(event) => setDraft((current) => ({ ...current, [key]: event.target.value }))}
                />
              )}
            </label>
          )
        })}
      </VStack>
      {spec.data.length ? (
        <VStack gap={6}>
          <Text size="caption" tone="secondary" weight="medium">{tr('Filled by the computer')}</Text>
          <ChoicePicker
            selection={functionName}
            options={[{ id: '', label: tr('Nothing') }, ...spec.data.map((name) => ({ id: name, label: name }))]}
            onChange={setFunctionName}
          />
          <Text size="caption2" tone="tertiary">
            {tr('The computer fills it when the card is shown.')}
          </Text>
        </VStack>
      ) : null}
      <VStack gap={6}>
        <Text size="caption" tone="secondary" weight="medium">{tr('Preview')}</Text>
        <div className="component-preview">
          <ComponentPreview card={preview} />
        </div>
      </VStack>
      {error ? <Text size="caption" tone="attention" className="declarative-text">{error}</Text> : null}
      {checked && !error ? (
        <HStack gap={6}>
          <Icon name="checkmark.circle.fill" size={12} color="var(--accent)" />
          <Text size="caption" tone="secondary">{tr('Passes the catalogue.')}</Text>
        </HStack>
      ) : null}
      <HStack gap={8} wrap>
        <Button kind="secondary" disabled={busy !== null} onClick={() => void run('preview')}>
          {busy === 'preview' ? <Spinner size={12} /> : tr('Check with the host')}
        </Button>
        <Button kind="primary" disabled={busy !== null} onClick={() => void run('save')}>
          {busy === 'save' ? <Spinner size={12} /> : instance ? tr('Save card') : tr('Save to the gallery')}
        </Button>
        <Switch on={published} onChange={setPublished}>
          <Text size="callout">{tr('Show every bot')}</Text>
        </Switch>
      </HStack>
    </VStack>
  )
}
