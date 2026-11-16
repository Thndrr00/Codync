import { useState } from 'react'
import type { Entry } from '@shared/models'
import {
  componentAnswers,
  componentComplete,
  componentFields,
  componentLabel,
  componentValues,
  trendArrow,
  type ChecklistItem,
  type ComponentCard,
  type ComponentCell,
  type ComponentField,
  type TimelineEvent,
} from '@shared/components.ts'
import { Button, ChoicePicker, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import './component-card.css'

/**
 * One of Codync's own gallery cards (host catalogue version 1). The host checked it before it was
 * posted, so this only maps a catalogue name to the controls the desktop already has; a name this
 * build doesn't know draws nothing.
 */
export function ComponentCardView({ entry, answering, respond }: {
  entry: Entry
  /** The option whose answer is on its way, if any. */
  answering: string | null
  respond: (option: string | null, values?: Record<string, string | number | boolean>) => void
}) {
  const card = entry.data.component
  if (!card) return null
  return <CardBody key={entry.id} card={card} entry={entry} answering={answering} respond={respond} />
}

function CardBody({ card, entry, answering, respond }: {
  card: ComponentCard
  entry: Entry
  answering: string | null
  respond: (option: string | null, values?: Record<string, string | number | boolean>) => void
}) {
  const tr = useTr()
  const fields = componentFields(card)
  const [values, setValues] = useState(() => ({ ...componentValues(card), ...(entry.data.values ?? {}) }))
  const pending = entry.data.status === 'pending'
  const ready = !pending || componentComplete(card, values)
  const title = card.title ?? tr(componentLabel(card.name))
  return (
    <div className="agent-inset">
      <VStack gap={10} className={`permission-card component-card ${card.name}`}>
        <HStack gap={8}>
          <Text size="headline" tone="text">{title}</Text>
          {pending ? <span className="permission-card-dot" /> : null}
        </HStack>
        {card.props.caption ? <Text size="caption" tone="secondary">{card.props.caption}</Text> : null}
        <ComponentBody card={card} values={values} edit={(id, value) => setValues((current) => ({ ...current, [id]: value }))} pending={pending} />
        {card.source ? (
          <HStack gap={4}>
            <Icon name="desktopcomputer" size={10} color="var(--secondary)" />
            <Text size="caption2" tone="tertiary">{tr('Filled by the computer')}</Text>
          </HStack>
        ) : null}
        {pending ? (
          <HStack gap={8} wrap>
            <Button
              kind="primary"
              disabled={answering !== null || !ready}
              onClick={() => respond('submit', fields.length ? componentAnswers(card, values) : undefined)}
            >
              {card.props.submit ?? tr('Send')}
            </Button>
            {ready ? null : <Text size="caption" tone="tertiary">{tr('Fill in the required fields to continue.')}</Text>}
          </HStack>
        ) : entry.data.status === 'answered' ? (
          <Text size="caption" tone="secondary">{tr('Answers sent')}</Text>
        ) : null}
      </VStack>
    </div>
  )
}

/** A card's body alone, for a preview outside the chat (the gallery and the playground). */
export function ComponentPreview({ card }: { card: ComponentCard }) {
  const tr = useTr()
  return (
    <VStack gap={10} className={`permission-card component-card ${card.name}`}>
      <Text size="headline" tone="text">{card.title ?? tr(componentLabel(card.name))}</Text>
      {card.props.caption ? <Text size="caption" tone="secondary">{card.props.caption}</Text> : null}
      <Body card={card} values={componentValues(card)} edit={() => {}} pending={false} />
    </VStack>
  )
}

function Body({ card, values, edit, pending }: {
  card: ComponentCard
  values: Record<string, string | number | boolean>
  edit: (id: string, value: string | number | boolean) => void
  pending: boolean
}) {
  const props = card.props
  switch (card.name) {
    case 'metric':
      return <Metric props={props} />
    case 'comparison':
      return <Grid columns={props.columns ?? []} rows={props.rows ?? []} highlight />
    case 'timeline':
      return <Timeline events={props.events ?? []} />
    case 'table':
      return props.rows?.length ? <Grid columns={props.columns ?? []} rows={props.rows} /> : <Empty />
    case 'checklist':
      return props.items?.length ? <Checklist items={props.items} /> : <Empty />
    case 'form':
      return (
        <VStack gap={8}>
          {componentFields(card).map((field) => (
            <FieldRow key={field.id} field={field} value={values[field.id]} edit={edit} pending={pending} />
          ))}
        </VStack>
      )
    default:
      // The catalogue is versioned with the app: a component this build doesn't know renders nothing.
      return null
  }
}

function Empty() {
  const tr = useTr()
  return <Text size="caption" tone="tertiary">{tr('Nothing to show yet.')}</Text>
}

function Metric({ props }: { props: ComponentCard['props'] }) {
  const trend = props.trend ?? 'flat'
  return (
    <VStack gap={4}>
      <Text size="caption" tone="secondary" weight="medium">{props.label}</Text>
      <HStack gap={8} align="baseline">
        <Text size="title2" tone="text" weight="semibold" className="component-metric-value">{props.value}</Text>
        {props.change ? (
          <Text size="footnote" tone={trend === 'down' ? 'attention' : 'secondary'} className={`component-trend ${trend}`}>
            {trendArrow(trend)} {props.change}
          </Text>
        ) : null}
      </HStack>
      {props.series?.length ? <Sparkline values={props.series} /> : null}
    </VStack>
  )
}

/** The metric's sparkline: a quiet line under the number, the theme's accent. */
function Sparkline({ values }: { values: number[] }) {
  const peak = Math.max(...values.map(Math.abs), 1)
  const step = values.length > 1 ? 1 / (values.length - 1) : 0
  const points = values.map((value, index) => `${(index * step * 100).toFixed(2)},${(36 - (Math.abs(value) / peak) * 32).toFixed(2)}`).join(' ')
  return (
    <div className="component-sparkline" role="img" aria-label={values.join(', ')}>
      <svg viewBox="0 0 100 40" preserveAspectRatio="none">
        <polyline points={points} fill="none" stroke="var(--accent)" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
      </svg>
    </div>
  )
}

function Grid({ columns, rows, highlight }: { columns: string[]; rows: ComponentCell[][]; highlight?: boolean }) {
  return (
    <div className="component-grid-wrap">
      <table className={`component-grid ${highlight ? 'comparison' : ''}`}>
        <thead>
          <tr>{columns.map((column, index) => <th key={index} scope="col">{column}</th>)}</tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr key={index}>
              {row.map((cell, column) => (
                <td key={column} className={highlight && column > 0 ? 'side' : undefined}>{cell === null ? '' : String(cell)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Timeline({ events }: { events: TimelineEvent[] }) {
  if (!events.length) return <Empty />
  return (
    <VStack gap={0} className="component-timeline">
      {events.map((event, index) => (
        <HStack key={index} gap={10} align="flex-start" className={`component-event ${event.tone ?? 'info'}`}>
          <VStack gap={0} align="center" className="component-event-rail">
            <span className={`component-event-dot ${event.tone ?? 'info'}`} />
            {index < events.length - 1 ? <span className="component-event-line" /> : null}
          </VStack>
          <VStack gap={2} grow className="component-event-body">
            <HStack gap={8}>
              <Text size="caption" tone="tertiary">{event.at}</Text>
              <Text size="callout" tone="text" weight="medium">{event.title}</Text>
            </HStack>
            {event.detail ? <Text size="footnote" tone="secondary" className="component-event-detail">{event.detail}</Text> : null}
          </VStack>
        </HStack>
      ))}
    </VStack>
  )
}

function Checklist({ items }: { items: ChecklistItem[] }) {
  return (
    <VStack gap={6}>
      {items.map((item, index) => (
        <HStack key={index} gap={8} align="flex-start">
          <Icon
            name={item.done ? 'checkmark.circle.fill' : 'circle'}
            size={13}
            color={item.done ? 'var(--accent)' : 'var(--secondary)'}
          />
          <VStack gap={2} grow>
            <Text size="callout" tone={item.done ? 'secondary' : 'text'} className={item.done ? 'component-item-done' : undefined}>
              {item.label}
            </Text>
            {item.note ? <Text size="caption" tone="tertiary">{item.note}</Text> : null}
          </VStack>
        </HStack>
      ))}
    </VStack>
  )
}

function FieldRow({ field, value, edit, pending }: {
  field: ComponentField
  value: string | number | boolean | undefined
  edit: (id: string, value: string | number | boolean) => void
  pending: boolean
}) {
  const tr = useTr()
  if (field.type === 'toggle') {
    return (
      <Switch on={value === true} onChange={(on) => edit(field.id, on)} disabled={!pending}>
        <Text size="callout">{field.label}</Text>
      </Switch>
    )
  }
  const label = (
    <HStack gap={4}>
      <Text size="footnote" tone="secondary" weight="medium">{field.label}</Text>
      {field.required ? <Text size="caption2" tone="tertiary">*</Text> : null}
    </HStack>
  )
  if (field.type === 'select') {
    return (
      <VStack gap={6}>
        {label}
        <ChoicePicker
          selection={String(value ?? '')}
          options={(field.options ?? []).map((option) => ({ id: option.value, label: option.label }))}
          onChange={(choice) => edit(field.id, choice)}
        />
      </VStack>
    )
  }
  const shared = {
    'aria-label': field.label,
    placeholder: field.placeholder ?? tr('Optional'),
    value: String(value ?? ''),
    disabled: !pending,
  }
  return (
    <VStack gap={6}>
      {label}
      {field.type === 'multiline' ? (
        <textarea
          {...shared}
          rows={3}
          className="declarative-input declarative-textarea"
          onChange={(event) => edit(field.id, event.target.value)}
        />
      ) : (
        <input
          {...shared}
          className="declarative-input"
          type={field.type === 'text' ? 'text' : field.type}
          inputMode={field.type === 'number' ? 'decimal' : undefined}
          onChange={(event) =>
            edit(
              field.id,
              field.type === 'number' && event.target.value !== '' ? Number(event.target.value) : event.target.value,
            )
          }
        />
      )}
    </VStack>
  )
}
