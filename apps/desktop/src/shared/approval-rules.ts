// A bot's approval rules and what the host's tester says about a command (`testApprovalRule`): the host decides, this
// only words its answer. See docs/features/approvals.md.

export type RuleDecision = 'allow' | 'ask' | 'deny'

export interface ApprovalRule {
  pattern: string
  /** Only requests of this tool kind; any kind when absent. */
  kind?: string | null
  decision: RuleDecision
}

/** The host's answer for one command (`approval_rules::Evaluation`). */
export type ApprovalEvaluation = (
  | { outcome: 'forbidden'; pattern: string }
  | { outcome: 'rule'; index: number; decision: RuleDecision; pattern: string }
  | { outcome: 'setting'; permission: 'ask' | 'auto' }
) & {
  /** The command runs several commands, so no `allow` rule approves it. */
  chained: boolean
  /** The index of an `allow` rule that matched but was passed over for that reason. */
  allowPassedOver: number | null
}

/** The tool kinds a rule can be limited to. */
export const RULE_KINDS = ['execute', 'edit', 'delete', 'move', 'read', 'search', 'fetch', 'think', 'other'] as const

/** At most this many rules of this many characters (the host refuses more). */
export const MAX_RULES = 100
export const MAX_PATTERN = 500

/** What an `allow` rule never approves, shown beside the editor. */
export const CHAINING_WARNING =
  'An allow rule never approves a command that runs several (; && || | ` $( > < or a new line): such a command asks instead.'

/** One sentence for the tester's answer; rule numbers count from 1, as the list shows them. */
export function describeEvaluation(e: ApprovalEvaluation): string {
  const passed = e.allowPassedOver === null ? '' : ` Rule ${e.allowPassedOver + 1} would allow it, but it runs several commands.`
  switch (e.outcome) {
    case 'forbidden':
      return `Never allowed: it matches “${e.pattern}” in the list of forbidden commands, whatever the rules say.`
    case 'rule': {
      const verb = e.decision === 'allow' ? 'Allowed' : e.decision === 'deny' ? 'Denied' : 'Asks you'
      return `${verb} by rule ${e.index + 1} (“${e.pattern}”).`
    }
    case 'setting':
      return `No rule matches: ${e.permission === 'auto' ? 'it is approved automatically' : 'it asks you'}, as the bot's Approvals setting says.${passed}`
  }
}

/** The rules with the one at `from` moved to `to` (the first match decides, so order matters). */
export function moveRule(rules: ApprovalRule[], from: number, to: number): ApprovalRule[] {
  if (from === to || from < 0 || to < 0 || from >= rules.length || to >= rules.length) return rules
  const next = [...rules]
  const [moved] = next.splice(from, 1)
  next.splice(to, 0, moved)
  return next
}

/** The rules a draft can save: blank patterns are dropped, patterns trimmed. */
export function cleanRules(rules: ApprovalRule[]): ApprovalRule[] {
  return rules
    .map((rule) => ({ ...rule, pattern: rule.pattern.trim(), kind: rule.kind || null }))
    .filter((rule) => rule.pattern.length > 0)
}

/** Why the rules can't be saved yet, or null. */
export function rulesProblem(rules: ApprovalRule[]): string | null {
  if (rules.length > MAX_RULES) return `A bot keeps at most ${MAX_RULES} approval rules.`
  if (rules.some((rule) => rule.pattern.length > MAX_PATTERN)) return `A rule's pattern is at most ${MAX_PATTERN} characters.`
  return null
}
