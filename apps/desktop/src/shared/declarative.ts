/**
 * A declarative UI block a bot shows in its chat (card K3). The host validates it against this
 * catalogue before it is posted; clients only render what is in the union below.
 */

/** The only schema version clients know. */
export const DECLARATIVE_VERSION = 1

export type BlockFieldType = 'text' | 'multiline' | 'number' | 'date' | 'time' | 'select' | 'toggle'

export interface BlockFieldOption {
  value: string
  label: string
}

export interface BlockField {
  id: string
  label: string
  type: BlockFieldType
  required?: boolean
  placeholder?: string
  options?: BlockFieldOption[]
  /** A starting value the agent suggested; the person may change it before sending. */
  value?: string | number | boolean | null
}

export interface BlockAction {
  id: string
  label: string
  /** primary | secondary | destructive */
  kind: string
}

export type BlockComponent =
  | { type: 'heading'; text: string }
  | { type: 'text'; text: string }
  | { type: 'divider' }
  | { type: 'callout'; tone: 'info' | 'warning'; text: string }
  | { type: 'field'; field: BlockField }
  | { type: 'table'; columns: string[]; rows: (string | number | boolean | null)[][] }
  | { type: 'chart'; chartType: 'bar' | 'line'; labels: string[]; values: number[] }

export interface DeclarativeBlock {
  version: number
  title: string
  body: BlockComponent[]
  actions: BlockAction[]
}

/** The form's answers, by field id, in the shape the host's answer path takes. */
export type BlockValues = Record<string, string | number | boolean>

/** Every field in a block, in body order. */
export function blockFields(block: DeclarativeBlock): BlockField[] {
  return block.body.flatMap((component) => (component.type === 'field' ? [component.field] : []))
}

/** The form's first values: what the agent suggested, else an empty one of each field's type. */
export function initialValues(block: DeclarativeBlock): Record<string, string | number | boolean> {
  const values: Record<string, string | number | boolean> = {}
  for (const field of blockFields(block)) {
    const value = field.value
    if (field.type === 'toggle') values[field.id] = value === true
    else if (field.type === 'number') values[field.id] = typeof value === 'number' ? value : ''
    else values[field.id] = typeof value === 'string' ? value : ''
  }
  return values
}

/** The answers to send: numbers that were left blank are left out, so the host can require them. */
export function answersOf(block: DeclarativeBlock, values: Record<string, string | number | boolean>): BlockValues {
  const answers: BlockValues = {}
  for (const field of blockFields(block)) {
    const value = values[field.id]
    if (field.type === 'number') {
      if (typeof value === 'number' && Number.isFinite(value)) answers[field.id] = value
    } else if (field.type === 'toggle') {
      answers[field.id] = value === true
    } else if (typeof value === 'string' && value.trim() !== '') {
      answers[field.id] = value.trim()
    }
  }
  return answers
}

/** Whether every required field has an answer, so the action buttons can stay disabled until then. */
export function complete(block: DeclarativeBlock, values: Record<string, string | number | boolean>): boolean {
  return blockFields(block).every((field) => {
    if (!field.required) return true
    const value = values[field.id]
    if (field.type === 'toggle') return true
    if (field.type === 'number') return typeof value === 'number' && Number.isFinite(value)
    return typeof value === 'string' && value.trim() !== ''
  })
}

/** A bar's height as a fraction of the chart, with a small floor so a zero still shows. */
export function barFraction(value: number, values: number[]): number {
  const peak = Math.max(...values.map(Math.abs), 1)
  return Math.max(Math.abs(value) / peak, 0.02)
}

/** A line chart's points on a 0..1 grid, `y` measured from the bottom. */
export function linePoints(values: number[]): { x: number; y: number }[] {
  const peak = Math.max(...values.map(Math.abs), 1)
  const step = values.length > 1 ? 1 / (values.length - 1) : 0
  return values.map((value, index) => ({ x: index * step, y: Math.abs(value) / peak }))
}
