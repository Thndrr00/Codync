/**
 * Codync's component gallery (host catalogue version 1): the cards a bot shows in its chat and the
 * gallery/playground draw. The host validates every card against this catalogue; a client only ever
 * renders the union below, one name at a time, with its own controls — nothing here is HTML, a script
 * or a React component.
 */

/** The catalogue version clients know. */
export const COMPONENT_VERSION = 1

export type ComponentName = 'metric' | 'comparison' | 'timeline' | 'table' | 'checklist' | 'form'

/** One timeline point. */
export interface TimelineEvent {
  at: string
  title: string
  detail?: string
  /** info | warning | done */
  tone?: string
}

/** One checklist row. */
export interface ChecklistItem {
  label: string
  done: boolean
  note?: string
}

/** A cell of a table or comparison: plain text, a number, true/false, or empty. */
export type ComponentCell = string | number | boolean | null

/** A form card's field, the same shape a declarative block's field has. */
export interface ComponentField {
  id: string
  label: string
  /** text | multiline | number | date | time | select | toggle */
  type: string
  required?: boolean
  placeholder?: string
  options?: { value: string; label: string }[]
}

/** The props a card carries, by component. */
export interface ComponentProps {
  /** metric */
  label?: string
  value?: string
  change?: string
  /** up | down | flat */
  trend?: string
  series?: number[]
  /** comparison and table */
  columns?: string[]
  rows?: ComponentCell[][]
  /** timeline */
  events?: TimelineEvent[]
  /** checklist */
  items?: ChecklistItem[]
  /** form */
  fields?: ComponentField[]
  submit?: string
  /** Every component. */
  caption?: string
}

/** The card document on an entry's `data`. */
export interface ComponentCard {
  version: number
  name: ComponentName | string
  title?: string
  props: ComponentProps
  /** The data function the host resolved some props from, when it did. */
  source?: string
}

/** One catalogue component, as `listComponents` describes it. */
export interface CatalogueComponent {
  name: string
  version: number
  summary: string
  /** The props as JSON Schema, for the playground's form. */
  schema: {
    type: string
    additionalProperties?: boolean
    properties: Record<string, {
      type?: string | string[]
      title?: string
      description?: string
      enum?: string[]
      maxLength?: number
      minItems?: number
      maxItems?: number
      items?: { type?: string | string[]; maxLength?: number }
    }>
    required: string[]
  }
  /** The data functions this component may name. */
  data: string[]
  /** Props that pass the schema, for the gallery's preview. */
  sample: ComponentProps
  /** Whether the bot the gallery was read for may use it. */
  available?: boolean
}

/** A card the playground saved. */
export interface ComponentInstance {
  id: string
  name: string
  version: number
  title: string
  props: ComponentProps
  data: { function: string; limit?: number }[]
  author?: string | null
  published: boolean
  createdAt: number
  updatedAt: number
  /** Whether the card still passes the catalogue (a host update can change a component's props). */
  valid?: boolean
}

/** One bot's say over the gallery: the components it may not use. */
export interface ComponentBot {
  id: string
  name: string
  /** agent | group */
  kind: string
  withheld: string[]
}

/** The whole gallery, as `listComponents` answers it. */
export interface ComponentGallery {
  version: number
  components: CatalogueComponent[]
  instances: ComponentInstance[]
  access: { bot: string; component: string; published: boolean; updatedAt: number }[]
  bots: ComponentBot[]
}

/** The component's name in the person's language; untranslated names fall back to the catalogue's. */
export function componentLabel(name: string): string {
  switch (name) {
    case 'metric':
      return 'Metric'
    case 'comparison':
      return 'Comparison'
    case 'timeline':
      return 'Timeline'
    case 'table':
      return 'Table'
    case 'checklist':
      return 'Checklist'
    case 'form':
      return 'Form'
    default:
      return 'Card'
  }
}

/** A metric's trend as an arrow, or nothing when the host sent none. */
export function trendArrow(trend: string | undefined): string {
  if (trend === 'up') return '↑'
  if (trend === 'down') return '↓'
  return '→'
}

/** Every field of a form card, in order. */
export function componentFields(card: ComponentCard): ComponentField[] {
  return card.props.fields ?? []
}

/** A card's starting values: none invented, numbers left blank so the host can require them. */
export function componentValues(card: ComponentCard): Record<string, string | number | boolean> {
  const values: Record<string, string | number | boolean> = {}
  for (const field of componentFields(card)) values[field.id] = field.type === 'toggle' ? false : ''
  return values
}

/** Whether every required field is filled, so the card's button can stay disabled until then. */
export function componentComplete(
  card: ComponentCard,
  values: Record<string, string | number | boolean>,
): boolean {
  return componentFields(card).every((field) => {
    if (!field.required) return true
    const value = values[field.id]
    if (field.type === 'toggle') return true
    if (field.type === 'number') return typeof value === 'number' && Number.isFinite(value)
    return typeof value === 'string' && value.trim() !== ''
  })
}

/** The answers to send: blank values are left out, so the host can require them. */
export function componentAnswers(
  card: ComponentCard,
  values: Record<string, string | number | boolean>,
): Record<string, string | number | boolean> {
  const answers: Record<string, string | number | boolean> = {}
  for (const field of componentFields(card)) {
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

/** A prop's editor in the playground, from its JSON Schema entry. */
export type PropEditor = 'text' | 'number' | 'toggle' | 'choice' | 'rows' | 'list' | 'numbers' | 'json'

export function propKind(schema: { type?: string | string[]; enum?: string[]; items?: { type?: string | string[] } }): PropEditor {
  if (schema.enum) return 'choice'
  const type = Array.isArray(schema.type) ? schema.type[0] : schema.type
  if (type === 'number' || type === 'integer') return 'number'
  if (type === 'boolean') return 'toggle'
  if (type === 'array') {
    const item = Array.isArray(schema.items?.type) ? schema.items?.type[0] : schema.items?.type
    if (item === 'number') return 'numbers'
    if (item === 'object') return 'json'
    return 'list'
  }
  return 'text'
}

/** A prop's value as the playground's editor shows it. */
export function propText(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (Array.isArray(value)) {
    if (value.some((entry) => entry !== null && typeof entry === 'object')) return JSON.stringify(value, null, 2)
    if (value.some((entry) => Array.isArray(entry))) {
      return value
        .map((row) =>
          Array.isArray(row) ? row.map((cell) => (cell === null ? '' : String(cell))).join(' | ') : String(row),
        )
        .join('\n')
    }
    return value.map((entry) => (entry === null ? '' : String(entry))).join(', ')
  }
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

/** What the playground's editor means, in the shape the host checks. */
export function parsePropText(kind: PropEditor, text: string): unknown {
  const trimmed = text.trim()
  if (kind === 'number') {
    const number = Number.parseFloat(text.replaceAll(',', '.'))
    return Number.isFinite(number) ? number : text
  }
  if (kind === 'toggle') return trimmed === 'true'
  if (kind === 'json') {
    try {
      return JSON.parse(text)
    } catch {
      // The host names the field and what it expects; the playground shows that reason.
      return text
    }
  }
  if (kind === 'numbers') {
    return text
      .split(',')
      .map((entry) => Number.parseFloat(entry.trim()))
      .filter((number) => Number.isFinite(number))
  }
  if (kind === 'rows') {
    return text
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line !== '')
      .map((line) =>
        line.split('|').map((cell) => {
          const entry = cell.trim()
          if (entry === '') return null
          const number = Number(entry)
          if (Number.isFinite(number)) return number
          if (entry === 'true') return true
          if (entry === 'false') return false
          return entry
        }),
      )
  }
  if (kind === 'list') {
    return text
      .split(',')
      .map((entry) => entry.trim())
      .filter((entry) => entry !== '')
  }
  return text
}
