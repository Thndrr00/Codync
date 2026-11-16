import { useState } from 'react'
import type { Entry } from '@shared/models'
import {
  answersOf,
  barFraction,
  blockFields,
  complete,
  initialValues,
  linePoints,
  type BlockComponent,
  type BlockField,
  type DeclarativeBlock,
} from '@shared/declarative.ts'
import { Button, ChoicePicker, Hairline, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import './declarative-card.css'

/**
 * A declarative UI block the bot shows (card K3): the host checked it against Codync's catalogue and
 * posted it as an approval card, so answering it uses the same path as every other approval.
 */
export function DeclarativeCard({ entry, answering, respond }: {
  entry: Entry
  /** The option whose answer is on its way, if any. */
  answering: string | null
  respond: (option: string | null, values?: Record<string, string | number | boolean>) => void
}) {
  const block = entry.data.block
  if (!block) return null
  return <BlockCard key={entry.id} block={block} entry={entry} answering={answering} respond={respond} />
}

function BlockCard({ block, entry, answering, respond }: {
  block: DeclarativeBlock
  entry: Entry
  answering: string | null
  respond: (option: string | null, values?: Record<string, string | number | boolean>) => void
}) {
  const tr = useTr()
  const fields = blockFields(block)
  const [values, setValues] = useState(() => ({ ...initialValues(block), ...(entry.data.values ?? {}) }))
  const pending = entry.data.status === 'pending'
  const busy = answering !== null
  const ready = pending && complete(block, values)
  const edit = (id: string, value: string | number | boolean) => setValues((current) => ({ ...current, [id]: value }))
  return (
    <div className="agent-inset">
      <VStack gap={10} className="permission-card declarative-card">
        <HStack gap={8}>
          <Text size="headline" tone="text">{block.title}</Text>
          {pending ? <span className="permission-card-dot" /> : null}
        </HStack>
        {block.body.map((component, index) => (
          <BlockRow key={index} component={component} values={values} edit={edit} pending={pending} />
        ))}
        {pending && fields.some((field) => field.required) && !ready ? (
          <Text size="caption" tone="tertiary">{tr('Fill in the required fields to continue.')}</Text>
        ) : null}
        {pending ? (
          <HStack gap={8} wrap>
            {block.actions.map((action) => (
              <Button
                key={action.id}
                kind={action.kind === 'primary' ? 'primary' : 'secondary'}
                disabled={busy || !ready}
                onClick={() => respond(action.id, fields.length ? answersOf(block, values) : undefined)}
              >
                {action.label}
              </Button>
            ))}
            {block.actions.some((action) => action.kind === 'destructive') ? null : (
              <Button kind="secondary" disabled={busy} onClick={() => respond(null)}>{tr('Cancel')}</Button>
            )}
          </HStack>
        ) : block.actions.length ? (
          <Text size="caption" tone="secondary">
            {entry.data.status === 'answered' ? tr('Answers sent') : tr('Cancel')}
          </Text>
        ) : null}
      </VStack>
    </div>
  )
}

function BlockRow({ component, values, edit, pending }: {
  component: BlockComponent
  values: Record<string, string | number | boolean>
  edit: (id: string, value: string | number | boolean) => void
  pending: boolean
}) {
  switch (component.type) {
    case 'heading':
      return <Text size="body" weight="semibold" tone="text">{component.text}</Text>
    case 'text':
      return <Text size="callout" tone="secondary" className="declarative-text">{component.text}</Text>
    case 'divider':
      return <Hairline />
    case 'callout':
      return (
        <HStack align="flex-start" gap={8} className={`declarative-callout ${component.tone}`}>
          <Icon name={component.tone === 'warning' ? 'exclamationmark.triangle.fill' : 'info.circle'} size={12} color={component.tone === 'warning' ? 'var(--warning, #ff9f0a)' : 'var(--accent)'} />
          <Text size="footnote" tone="text" className="declarative-text">{component.text}</Text>
        </HStack>
      )
    case 'field':
      return <FieldRow field={component.field} value={values[component.field.id]} edit={edit} pending={pending} />
    case 'table':
      return (
        <div className="declarative-table-wrap">
          <table className="declarative-table">
            <thead>
              <tr>{component.columns.map((column, index) => <th key={index} scope="col">{column}</th>)}</tr>
            </thead>
            <tbody>
              {component.rows.map((row, index) => (
                <tr key={index}>{row.map((cell, column) => <td key={column}>{cell === null ? '' : String(cell)}</td>)}</tr>
              ))}
            </tbody>
          </table>
        </div>
      )
    case 'chart':
      return <Chart component={component} />
    default:
      // The host's catalogue is versioned with the app: a component this build doesn't know renders nothing.
      return null
  }
}

function FieldRow({ field, value, edit, pending }: {
  field: BlockField
  value: string | number | boolean | undefined
  edit: (id: string, value: string | number | boolean) => void
  pending: boolean
}) {
  const id = `block-field-${field.id}`
  const label = (
    <HStack gap={4}>
      <Text size="footnote" tone="secondary" weight="medium">{field.label}</Text>
      {field.required ? <Text size="caption2" tone="tertiary">*</Text> : null}
    </HStack>
  )
  if (field.type === 'toggle') {
    return (
      <div className="declarative-field">
        <Switch on={value === true} onChange={(on) => edit(field.id, on)} disabled={!pending}>
          <Text size="callout">{field.label}</Text>
        </Switch>
      </div>
    )
  }
  if (field.type === 'select') {
    return (
      <VStack gap={6} className="declarative-field">
        {label}
        <ChoicePicker
          selection={String(value ?? '')}
          options={(field.options ?? []).map((option) => ({ id: option.value, label: option.label }))}
          onChange={(choice) => edit(field.id, choice)}
        />
      </VStack>
    )
  }
  const input = field.type === 'multiline' ? (
    <textarea
      id={id}
      aria-label={field.label}
      className="declarative-input declarative-textarea"
      rows={3}
      placeholder={field.placeholder}
      value={String(value ?? '')}
      disabled={!pending}
      onChange={(event) => edit(field.id, event.target.value)}
    />
  ) : (
    <input
      id={id}
      aria-label={field.label}
      className="declarative-input"
      type={field.type === 'text' ? 'text' : field.type}
      inputMode={field.type === 'number' ? 'decimal' : undefined}
      placeholder={field.placeholder}
      value={String(value ?? '')}
      disabled={!pending}
      onChange={(event) => edit(field.id, field.type === 'number' && event.target.value !== '' ? Number(event.target.value) : event.target.value)}
    />
  )
  return (
    <VStack gap={6} className="declarative-field">
      {label}
      {input}
    </VStack>
  )
}

/** A bar or line chart of at most 12 points, drawn with the theme's accent. */
function Chart({ component }: { component: Extract<BlockComponent, { type: 'chart' }> }) {
  const label = `${component.chartType} chart: ${component.labels.join(', ')}`
  if (component.chartType === 'line') {
    const points = linePoints(component.values)
      .map((point) => `${point.x * 100},${40 - point.y * 36}`)
      .join(' ')
    return (
      <div className="declarative-chart" role="img" aria-label={label}>
        <svg viewBox="0 0 100 40" preserveAspectRatio="none" className="declarative-line">
          <polyline points={points} fill="none" stroke="var(--accent)" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
        </svg>
        <HStack gap={4} className="declarative-chart-labels">
          {component.labels.map((text, index) => <Text key={index} size="caption2" tone="tertiary" grow>{text}</Text>)}
        </HStack>
      </div>
    )
  }
  return (
    <div className="declarative-chart" role="img" aria-label={label}>
      <HStack align="flex-end" gap={6} className="declarative-bars">
        {component.values.map((value, index) => (
          <VStack key={index} gap={4} align="center" grow className="declarative-bar">
            <div className="declarative-bar-fill" style={{ height: `${barFraction(value, component.values) * 100}%` }} title={`${component.labels[index]}: ${value}`} />
            <Text size="caption2" tone="tertiary" truncate>{component.labels[index]}</Text>
          </VStack>
        ))}
      </HStack>
    </div>
  )
}
