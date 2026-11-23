import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react'
import { IconButton } from '../../../components/Controls'
import { Text } from '../../../components/Text'
import {
  chipBefore, fieldTokens, fieldTrigger, insertFieldToken, matchingFields,
  type FieldTrigger, type FlowField,
} from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { FieldPicker } from './FieldPicker'

/**
 * A prompt, task, question or message box whose `{{name}}` fields show as chips. The textarea still
 * holds the exact saved text (`{{name}}`), so copy, paste and older hosts work as before; a mirror
 * under it paints the chips and their problems. Typing `{{` (or a single `{`) opens the field list,
 * arrows move, Enter or Tab inserts, and Backspace on a chip removes the whole field.
 */
export function FieldTextArea({ value, onChange, fields, problems = {}, disabled = false, placeholder, label, rows = 3, maxLength = 8000 }: {
  value: string
  onChange: (value: string) => void
  fields: readonly FlowField[]
  /** Problems by field name, shown in the warning token and on hover. */
  problems?: Record<string, string>
  disabled?: boolean
  placeholder?: string
  label: string
  rows?: number
  maxLength?: number
}) {
  const tr = useTr()
  const area = useRef<HTMLTextAreaElement>(null)
  const mirror = useRef<HTMLDivElement>(null)
  const lastEmitted = useRef(value)
  const caretAfter = useRef<number | null>(null)
  const buttonCaret = useRef(0)
  const [trigger, setTrigger] = useState<FieldTrigger | null>(null)
  const [active, setActive] = useState(0)
  /** A single `{` means a new line until a letter or an arrow makes the list an explicit choice. */
  const [navigated, setNavigated] = useState(false)
  const [insertOpen, setInsertOpen] = useState(false)

  const query = trigger?.query ?? ''
  const matches = (trigger || insertOpen) ? matchingFields(fields, query) : []
  const open = !!trigger && (trigger.single ? (query === '' || matches.length > 0) : true) || insertOpen

  // An outside save (or another step being selected) replaces the text: forget the old completion.
  useEffect(() => {
    if (value !== lastEmitted.current) {
      setTrigger(null)
      setInsertOpen(false)
      setActive(0)
      setNavigated(false)
    }
    lastEmitted.current = value
  }, [value])

  // Put the caret back after an insertion or a chip removal, once React has painted the new text.
  useLayoutEffect(() => {
    const at = caretAfter.current
    if (at === null) return
    area.current?.focus()
    area.current?.setSelectionRange(at, at)
    caretAfter.current = null
  }, [value])

  const pick = (name: string) => {
    const at = insertOpen ? buttonCaret.current : area.current?.selectionStart ?? value.length
    const next = insertFieldToken(value, at, name)
    lastEmitted.current = next.text
    caretAfter.current = next.caret
    onChange(next.text)
    setTrigger(null)
    setInsertOpen(false)
    setActive(0)
    setNavigated(false)
  }

  const changed = (text: string, caret: number) => {
    const next = fieldTrigger(text, caret)
    lastEmitted.current = text
    setTrigger(next)
    setActive(0)
    if (!next) setNavigated(false)
    else if (next.single && next.query) setNavigated(true)
    onChange(text)
  }

  const onKey = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    const caret = event.currentTarget.selectionStart
    if (event.key === 'Backspace' && !event.currentTarget.value.slice(0, caret).endsWith('{{')) {
      const chip = chipBefore(value, caret)
      if (chip) {
        event.preventDefault()
        const text = value.slice(0, chip.start) + value.slice(chip.end)
        lastEmitted.current = text
        caretAfter.current = chip.start
        onChange(text)
        setTrigger(null)
        return
      }
    }
    if (event.key === 'Escape') {
      setTrigger(null)
      setInsertOpen(false)
      return
    }
    if (!open || matches.length === 0) {
      // After a single `{`, Enter stays a new line until the list was used.
      if (event.key === 'Enter' && trigger?.single && !navigated) setTrigger(null)
      return
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      setNavigated(true)
      setActive((current) => (current + (event.key === 'ArrowDown' ? 1 : matches.length - 1)) % matches.length)
      return
    }
    if (event.key === 'Enter' && trigger?.single && query === '' && !navigated) {
      setTrigger(null)
      return
    }
    if (event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      pick(matches[Math.min(active, matches.length - 1)].field.name)
    }
  }

  return (
    <div className="flow-field-editor">
      <div className="flow-field-anchor">
        <div className={`flow-field-surface${disabled ? ' disabled' : ''}`}>
          <div ref={mirror} className="flow-field-mirror" aria-hidden>
            {fieldTokens(value).map((token, index) => token.kind === 'field'
              ? <span key={index} className={`flow-field-chip${problems[token.name] || problems['*'] ? ' problem' : ''}`}
                  title={problems[token.name] ?? problems['*']}>{token.raw}</span>
              : <span key={index}>{token.text}</span>)}
          </div>
          <textarea
            ref={area}
            className="flow-field-input"
            aria-label={tr(label)}
            placeholder={placeholder ? tr(placeholder) : undefined}
            value={value}
            rows={rows}
            maxLength={maxLength}
            disabled={disabled}
            spellCheck={false}
            onChange={(event) => changed(event.target.value, event.target.selectionStart)}
            onKeyDown={onKey}
            onScroll={(event) => {
              if (mirror.current) mirror.current.scrollTop = event.currentTarget.scrollTop
            }}
          />
        </div>
        {open ? (
          <div className="flow-field-menu">
            <FieldPicker
              fields={fields}
              query={query}
              active={Math.min(active, matches.length - 1)}
              onPick={pick}
              showSource={insertOpen}
            />
          </div>
        ) : null}
      </div>
      {fields.length > 0 ? (
        <div className="flow-field-tools">
          <IconButton
            title={tr('Insert field')}
            icon="plus"
            size={22}
            disabled={disabled}
            menuOpen={insertOpen}
            onClick={() => {
              buttonCaret.current = area.current?.selectionStart ?? value.length
              setTrigger(null)
              setInsertOpen(!insertOpen)
              setActive(0)
            }}
          />
          <Text size="caption" tone="secondary">{tr('Fields from earlier steps')}</Text>
        </div>
      ) : null}
    </div>
  )
}
