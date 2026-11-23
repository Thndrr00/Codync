import { Icon } from '../../../components/Icon'
import { Text } from '../../../components/Text'
import { matchingFields, type FlowField } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'

/** The name of a field's type in the field list (OpenBot's "One of" / "Whole number"…). */
export const FIELD_TYPE_LABEL: Record<FlowField['type'], string> = {
  text: 'Text',
  number: 'Number',
  integer: 'Whole number',
  yesNo: 'Yes or no',
  list: 'List',
  choice: 'One of',
}

/** What a field row shows after its name: its choices, or its type. */
const detailOf = (field: FlowField, tr: (text: string) => string) =>
  field.values.length > 0 ? field.values.join(', ') : tr(FIELD_TYPE_LABEL[field.type])

/**
 * The one-line-per-field list opened by `{{`, by a single `{` and by Insert field. Typed letters are
 * bold, prefix matches come first, and a row carries its giving step (Insert field) and its type or
 * choices. The caller owns the query and which row the arrow keys highlight.
 */
export function FieldPicker({ fields, query, active, onPick, showSource = false }: {
  fields: readonly FlowField[]
  query: string
  active: number
  onPick: (name: string) => void
  /** Insert field shows the step that gives each field, even without a query. */
  showSource?: boolean
}) {
  const tr = useTr()
  const matches = matchingFields(fields, query)
  if (matches.length === 0) {
    return (
      <div className="flow-field-picker flow-field-empty" role="status">
        <Text size="callout" tone="secondary">{tr('No matches')}</Text>
      </div>
    )
  }
  return (
    <div className="flow-field-picker" role="listbox" aria-label={tr('Fields from earlier steps')}>
      {matches.map((match, index) => {
        const detail = detailOf(match.field, tr)
        return (
          <button
            key={match.field.name}
            type="button"
            role="option"
            aria-selected={index === active}
            className={`flow-field-row${index === active ? ' active' : ''}`}
            // Keep the caret in the text being edited; clicking a row inserts its field.
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onPick(match.field.name)}
          >
            <Icon name={match.field.icon || 'bell'} size={13} />
            <span className="flow-field-name">
              {match.parts.map((part, partIndex) => part.match
                ? <Text key={partIndex} weight="semibold">{part.text}</Text>
                : <span key={partIndex}>{part.text}</span>)}
            </span>
            <Text size="caption" tone="secondary" truncate>
              {showSource ? tr('{step} · {detail}').replace('{step}', match.field.step || tr('Trigger')).replace('{detail}', detail) : detail}
            </Text>
          </button>
        )
      })}
    </div>
  )
}
