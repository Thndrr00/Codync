import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  answersOf,
  barFraction,
  blockFields,
  complete,
  initialValues,
  linePoints,
  type DeclarativeBlock,
} from './declarative.ts'

/** The block the host's acceptance example posts. */
const block: DeclarativeBlock = {
  version: 1,
  title: 'Book the trip',
  body: [
    { type: 'heading', text: 'Madrid' },
    { type: 'field', field: { id: 'nights', label: 'Nights', type: 'number', required: true, value: 2 } },
    { type: 'field', field: { id: 'room', label: 'Room', type: 'select', required: true, options: [{ value: 'single', label: 'Single' }, { value: 'double', label: 'Double' }] } },
    { type: 'field', field: { id: 'note', label: 'Note', type: 'multiline' } },
    { type: 'field', field: { id: 'late', label: 'Late arrival', type: 'toggle', value: true } },
    { type: 'table', columns: ['Item', 'Price'], rows: [['Room', 120]] },
    { type: 'chart', chartType: 'bar', labels: ['Mon', 'Tue'], values: [2, 5] },
  ],
  actions: [{ id: 'approve', label: 'Approve', kind: 'primary' }],
}

test('the form starts with what the agent suggested and no invented answers', () => {
  const values = initialValues(block)
  assert.equal(values.nights, 2)
  assert.equal(values.late, true)
  assert.equal(values.room, '')
  assert.equal(values.note, '')
})

test('every body field is found once, in the order it appears', () => {
  assert.deepEqual(blockFields(block).map((field) => field.id), ['nights', 'room', 'note', 'late'])
})

test('an incomplete form cannot be sent, a complete one sends only real answers', () => {
  assert.equal(complete(block, initialValues(block)), false)
  const filled = { ...initialValues(block), room: 'double', note: '  ' }
  assert.equal(complete(block, filled), true)
  assert.deepEqual(answersOf(block, filled), { nights: 2, room: 'double', late: true })
})

test('a blank number is left out instead of being sent as NaN', () => {
  const values = { ...initialValues(block), nights: '', room: 'single' }
  assert.equal(complete(block, values), false)
  assert.deepEqual(answersOf(block, values), { room: 'single', late: true })
})

test('charts normalize their points and keep zero-height bars visible', () => {
  assert.equal(barFraction(5, [2, 5]), 1)
  assert.equal(barFraction(0, [2, 5]), 0.02)
  assert.deepEqual(linePoints([0, 5]), [{ x: 0, y: 0 }, { x: 1, y: 1 }])
})
