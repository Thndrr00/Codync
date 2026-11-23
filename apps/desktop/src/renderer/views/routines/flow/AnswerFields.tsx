import { useState } from 'react'
import { Button, DropdownMenu } from '../../../components/Controls'
import { Icon } from '../../../components/Icon'
import { Text } from '../../../components/Text'
import { formatAnswerSpec, parseAnswerSpec, type AnswerFieldType, type AnswerFieldRow } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { FIELD_TYPE_LABEL } from './FieldPicker'

const TYPES: AnswerFieldType[] = ['text', 'number', 'integer', 'yesNo', 'list', 'choice']
/** A compact glyph before each type name; the name is the accessible label. */
const TYPE_GLYPH: Record<AnswerFieldType, string> = {
  text: 'T', number: '#', integer: '1', yesNo: '✓', list: '≡', choice: '·',
}

const compose = (rows: readonly AnswerFieldRow[], unknown: string) =>
  [formatAnswerSpec(rows), unknown.trim()].filter(Boolean).join('\n')

/** The Answer fields rows. The saved text stays `name: type`, so an older host reads it as before. */
export function AnswerFieldsPanel({ text, onSave }: { text: string; onSave: (text: string) => void }) {
  const tr = useTr()
  const initial = parseAnswerSpec(text)
  const [rows, setRows] = useState<AnswerFieldRow[]>(initial.rows)
  const [unknown, setUnknown] = useState(initial.unknown)
  const update = (index: number, change: Partial<AnswerFieldRow>) =>
    setRows((current) => current.map((row, at) => at === index ? { ...row, ...change } : row))
  return (
    <div className="flow-answer-rows">
      {rows.map((row, index) => (
        <div key={index} className="flow-answer-row">
          <DropdownMenu
            className="flow-answer-type"
            title={tr(FIELD_TYPE_LABEL[row.type])}
            label={tr(FIELD_TYPE_LABEL[row.type])}
            items={() => TYPES.map((type) => ({
              title: FIELD_TYPE_LABEL[type],
              selected: type === row.type,
              action: () => update(index, { type, values: type === 'choice' && row.values.length < 2 ? ['high', 'low'] : row.values }),
            }))}
          >
            <span aria-hidden>{TYPE_GLYPH[row.type]}</span>
          </DropdownMenu>
          <input
            className="routine-input flow-answer-name"
            value={row.name}
            aria-label={tr('Field name')}
            placeholder={tr('Name')}
            onChange={(event) => update(index, { name: event.target.value })}
          />
          {row.type === 'choice' ? (
            <input
              className="routine-input flow-answer-values"
              value={row.values.join(', ')}
              aria-label={tr('Values')}
              placeholder={tr('high, low')}
              onChange={(event) => update(index, { values: event.target.value.split(',').map((value) => value.trim()).filter(Boolean) })}
            />
          ) : (
            <Text size="caption" tone="secondary" className="flow-answer-detail">{tr(FIELD_TYPE_LABEL[row.type])}</Text>
          )}
          <button
            type="button"
            className={`flow-answer-optional${row.optional ? ' on' : ''}`}
            aria-label={tr('Optional')}
            aria-pressed={row.optional}
            title={tr('Optional')}
            onClick={() => update(index, { optional: !row.optional })}
          >?</button>
          <button
            type="button"
            className="flow-answer-remove"
            aria-label={tr('Remove field')}
            title={tr('Remove field')}
            onClick={() => setRows((current) => current.filter((_, at) => at !== index))}
          >
            <Icon name="trash" size={12} />
          </button>
        </div>
      ))}
      {unknown ? (
        <div className="flow-answer-unknown">
          <Text size="caption" tone="secondary">{tr('Lines the rows cannot show stay here.')}</Text>
          <textarea
            className="routine-input flow-answer-raw"
            aria-label={tr('Answer fields')}
            value={unknown}
            rows={2}
            onChange={(event) => setUnknown(event.target.value)}
          />
        </div>
      ) : null}
      <div className="flow-answer-actions">
        <Button kind="secondary" onClick={() => setRows((current) => [...current, { name: '', type: 'text', values: [], optional: false }])}>
          {tr('Add field')}
        </Button>
        <Text size="caption" tone="secondary" grow>{tr('Saved as “name: type”.')}</Text>
        <Button disabled={rows.some((row) => !row.name.trim())} onClick={() => onSave(compose(rows, unknown))}>{tr('Save')}</Button>
      </div>
    </div>
  )
}

/** The Check expression editor; the step itself no longer shows the answer or check under its prompt. */
export function CheckPanel({ text, onSave }: { text: string; onSave: (text: string) => void }) {
  const tr = useTr()
  const [draft, setDraft] = useState(text)
  return (
    <div className="flow-check-panel">
      <Text size="callout" tone="secondary">{tr('Join tests with and or or, for example “priority = high and count > 2”.')}</Text>
      <textarea
        className="routine-input flow-instruction"
        aria-label={tr('Check')}
        placeholder={tr('priority = high and count > 2')}
        value={draft}
        rows={3}
        onChange={(event) => setDraft(event.target.value)}
      />
      <div className="flow-answer-actions">
        <Button disabled={draft === text} onClick={() => onSave(draft)}>{tr('Save')}</Button>
      </div>
    </div>
  )
}

