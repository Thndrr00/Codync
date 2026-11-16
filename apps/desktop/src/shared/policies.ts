// This computer's policy rules (`docs/features/policy-engine.md`): a named CEL expression with an effect
// (`allow` or `deny`) and a message, its presets, and the what-if's answer. The host decides every answer;
// nothing here copies its logic.

export type PolicyEffect = 'allow' | 'deny'

/** One rule as it is saved. */
export interface Policy {
  id: string
  name: string
  expression: string
  effect: PolicyEffect
  message: string
  enabled: boolean
  /** 1 when it was first saved; one more on every change. */
  version: number
  updatedAt: number
}

/** A rule being written: saving one without an id adds it. */
export interface PolicyDraft {
  id?: string
  name: string
  expression: string
  effect: PolicyEffect
  message: string
  enabled: boolean
}

/** A starting point the host offers; it is a rule only once the person saves it. */
export interface PolicyPreset {
  id: string
  name: string
  about: string
  expression: string
  effect: PolicyEffect
  message: string
}

/** A variable a rule may use. */
export interface PolicyVariable {
  name: string
  about: string
}

export interface PolicyLimits {
  policies: number
  name: number
  expression: number
  message: number
}

export interface PoliciesReply {
  policies: Policy[]
  presets: PolicyPreset[]
  variables: PolicyVariable[]
  limits: PolicyLimits
  /** Whether any rule is switched on: until one is, the bots' approval rules decide. */
  enforcing: boolean
  /** An allow rule is saved: only what an allow rule names goes through. */
  allowlist: boolean
}

/** The rule that decided, as the host names it. */
export interface PolicyRule {
  id: string
  name: string
  effect: PolicyEffect
  index: number
  message?: string | null
}

/** One rule's own answer, as the what-if lists the candidates (`rules`). */
export interface PolicyRuleTrace {
  id: string
  name: string
  effect: PolicyEffect
  index: number
  /** Whether its expression held for this request. */
  matched: boolean
  /** Why it could not be tried; such a rule refuses a real request. */
  error?: string | null
}

/** What the policy list says about a proposed call (`previewPolicy`). */
export interface PolicyPreview {
  decision: 'allow' | 'deny' | 'unset'
  source: 'policy' | 'noMatch' | 'error' | 'unset'
  rule: PolicyRule | null
  allowlist: boolean
  refusal: string | null
  /** Every enabled rule's own answer, in the order they are tried. */
  rules?: PolicyRuleTrace[]
  /** What the rules read, with credentials masked. */
  variables: Record<string, unknown>
  explanation: string
}

export interface PolicyPreviewRequest {
  /** Tests a list that isn't saved yet; without it the saved rules are used. */
  policies?: PolicyDraft[]
  tool: string
  command?: string
  kind?: string
  intent?: string
  initiator?: 'person' | 'routine' | 'channel' | 'network'
  botId?: string
  filePath?: string
  fileOp?: string
  mcpServer?: string
  mcpTool?: string
  pageUrl?: string
  key?: string
}

/** The what-if's initiators, as `initiator.kind` spells them. */
export const INITIATORS = [
  { id: 'person', label: 'A person' },
  { id: 'routine', label: 'A routine' },
  { id: 'channel', label: 'A channel' },
  { id: 'network', label: 'Another computer' },
] as const

/** How a row names its effect. */
export function effectLabel(effect: PolicyEffect): string {
  return effect === 'allow' ? 'Allow' : 'Deny'
}
/** A preset as a new row, ready to edit before it is saved. */
export function fromPreset(preset: PolicyPreset): PolicyDraft {
  return { name: preset.name, expression: preset.expression, effect: preset.effect, message: preset.message, enabled: true }
}

/** A blank row. */
export function blankPolicy(): PolicyDraft {
  return { name: '', expression: '', effect: 'deny', message: '', enabled: true }
}

/** The draft with its text trimmed, and its empty name given one from its effect. */
export function cleanPolicy(draft: PolicyDraft): PolicyDraft {
  return {
    ...draft,
    name: draft.name.trim() || (draft.effect === 'allow' ? 'Allow rule' : 'Deny rule'),
    expression: draft.expression.trim(),
    message: draft.message.trim(),
  }
}

/** Why the rule can't be saved yet, or null. The host compiles it too, and says more when it can't. */
export function policyProblem(draft: PolicyDraft, limits: PolicyLimits): string | null {
  if (draft.name.trim().length > limits.name) return `A rule's name is at most ${limits.name} characters.`
  if (!draft.expression.trim()) return 'A rule needs an expression.'
  if (draft.expression.trim().length > limits.expression) return `A rule's expression is at most ${limits.expression} characters.`
  if (draft.message.trim().length > limits.message) return `A rule's message is at most ${limits.message} characters.`
  return null
}

/** The snapshot as rows an editor can show: `tool.name  shell`. */
export function variableRows(snapshot: Record<string, unknown>): { name: string; value: string }[] {
  const rows: { name: string; value: string }[] = []
  for (const [group, value] of Object.entries(snapshot ?? {})) {
    if (value === null || value === undefined) continue
    if (typeof value !== 'object') {
      rows.push({ name: group, value: String(value) })
      continue
    }
    for (const [field, inner] of Object.entries(value as Record<string, unknown>)) {
      rows.push({ name: `${group}.${field}`, value: inner === null || inner === undefined ? '' : String(inner) })
    }
  }
  return rows
}
