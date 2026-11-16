import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  componentAnswers,
  componentComplete,
  componentFields,
  componentLabel,
  componentValues,
  parsePropText,
  propKind,
  propText,
  trendArrow,
  type ComponentCard,
} from './components.ts'

const form: ComponentCard = {
  version: 1,
  name: 'form',
  title: 'Name the release',
  props: {
    fields: [
      { id: 'version', label: 'Version', type: 'text', required: true },
      { id: 'notes', label: 'Notes', type: 'multiline' },
    ],
    submit: 'Start the release',
  },
}

test('a form card asks only for what it declares, and sends only what was filled', () => {
  assert.deepEqual(componentFields(form).map((field) => field.id), ['version', 'notes'])
  const values = componentValues(form)
  assert.deepEqual(values, { version: '', notes: '' })
  assert.equal(componentComplete(form, values), false)
  assert.equal(componentComplete(form, { ...values, version: '3.4.0' }), true)
  assert.deepEqual(componentAnswers(form, { ...values, version: ' 3.4.0 ', notes: '   ' }), { version: '3.4.0' })
})

test('a component with no form never blocks a client on answers', () => {
  const metric: ComponentCard = { version: 1, name: 'metric', props: { label: 'Builds', value: '42' } }
  assert.deepEqual(componentFields(metric), [])
  assert.equal(componentComplete(metric, {}), true)
  assert.deepEqual(componentAnswers(metric, {}), {})
})

test('every catalogue name has a label and a fallback for one this build does not know', () => {
  for (const name of ['metric', 'comparison', 'timeline', 'table', 'checklist', 'form']) {
    assert.notEqual(componentLabel(name), 'Card')
  }
  assert.equal(componentLabel('iframe'), 'Card')
})

test('a trend shows an arrow, and an absent one points nowhere', () => {
  assert.equal(trendArrow('up'), '↑')
  assert.equal(trendArrow('down'), '↓')
  assert.equal(trendArrow('flat'), '→')
  assert.equal(trendArrow(undefined), '→')
})

test('the playground reads a schema entry as the editor its props need', () => {
  assert.equal(propKind({ type: 'string' }), 'text')
  assert.equal(propKind({ type: 'number' }), 'number')
  assert.equal(propKind({ type: 'boolean' }), 'toggle')
  assert.equal(propKind({ type: 'string', enum: ['up', 'down'] }), 'choice')
  assert.equal(propKind({ type: 'array', items: { type: 'string' } }), 'list')
  assert.equal(propKind({ type: 'array', items: { type: 'number' } }), 'numbers')
  assert.equal(propKind({ type: 'array', items: { type: 'object' } }), 'json')
})

test('a table’s rows round-trip through the playground’s lines, numbers kept as numbers', () => {
  const rows = [['Morning digest', 'Weekdays', 12], ['Weekly review', null, 3]]
  const text = propText(rows)
  assert.equal(text, 'Morning digest | Weekdays | 12\nWeekly review |  | 3')
  assert.deepEqual(parsePropText('rows', text), rows)
})

test('a list of names and a sparkline each round-trip', () => {
  assert.deepEqual(parsePropText('list', 'Plan A, Plan B'), ['Plan A', 'Plan B'])
  assert.equal(propText(['Plan A', 'Plan B']), 'Plan A, Plan B')
  assert.deepEqual(parsePropText('numbers', '12, 18, 15'), [12, 18, 15])
  assert.equal(propText([12, 18, 15]), '12, 18, 15')
})

test('timeline points are edited as their own JSON, and text that is not JSON stays for the host to refuse', () => {
  const events = [{ at: '09:12', title: 'Build started' }]
  assert.deepEqual(parsePropText('json', propText(events)), events)
  assert.equal(parsePropText('json', 'not json'), 'not json')
})
