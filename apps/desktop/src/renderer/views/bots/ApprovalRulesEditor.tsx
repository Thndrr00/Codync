import { useState } from 'react'
import { Button, ChoicePicker, IconButton } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import {
  CHAINING_WARNING,
  describeEvaluation,
  moveRule,
  RULE_KINDS,
  type ApprovalEvaluation,
  type ApprovalRule,
  type RuleDecision,
} from '@shared/approval-rules'
import { Field, AutoTextArea } from './parts'
import './approval-rules.css'

const DECISIONS: { id: RuleDecision; label: string }[] = [
  { id: 'allow', label: 'Allow' },
  { id: 'ask', label: 'Ask me' },
  { id: 'deny', label: 'Deny' },
]
const KINDS = [{ id: '', label: 'Any tool' }, ...RULE_KINDS.map((id) => ({ id: id as string, label: id }))]

/**
 * A bot's approval rules, first match first: add, edit, reorder, delete, and try a command against them. The host
 * decides what a rule does (`testApprovalRule`); nothing here copies its logic.
 */
export function ApprovalRulesEditor({ rules, onChange, test }: {
  rules: ApprovalRule[]
  onChange: (rules: ApprovalRule[]) => void
  /** Asks the host what the bot would do with `command` under `rules`; null when it can't say. */
  test: (command: string, rules: ApprovalRule[]) => Promise<ApprovalEvaluation | null>
}) {
  const [command, setCommand] = useState('')
  const [answer, setAnswer] = useState<string | null>(null)
  const replace = (index: number, patch: Partial<ApprovalRule>) =>
    onChange(rules.map((rule, i) => (i === index ? { ...rule, ...patch } : rule)))
  const run = async () => {
    if (!command.trim()) return
    const evaluation = await test(command, rules)
    setAnswer(evaluation ? describeEvaluation(evaluation) : "This computer's host can't test rules.")
  }
  return (
    <Field label="Approval rules" detail={`The first rule that matches decides, before the Approvals setting. ${CHAINING_WARNING} A * matches any text.`}>
      <VStack gap={8}>
        {rules.length === 0 ? <Text tone="secondary" size="footnote">No rules: the Approvals setting decides everything.</Text> : null}
        {rules.map((rule, index) => (
          <HStack key={index} gap={6} className="rule-row">
            <Text tone="tertiary" tabular className="rule-number">{index + 1}</Text>
            <ChoicePicker selection={rule.decision} options={DECISIONS} onChange={(decision) => replace(index, { decision })} />
            <ChoicePicker selection={rule.kind ?? ''} options={KINDS} onChange={(kind) => replace(index, { kind: kind || null })} />
            <AutoTextArea className="field-box rule-pattern" value={rule.pattern} placeholder="e.g. git status*" minRows={1} maxRows={3}
              onChange={(pattern) => replace(index, { pattern })} />
            <IconButton title="Move rule up" icon="chevron.up" disabled={index === 0} onClick={() => onChange(moveRule(rules, index, index - 1))} />
            <IconButton title="Move rule down" icon="chevron.down" disabled={index === rules.length - 1} onClick={() => onChange(moveRule(rules, index, index + 1))} />
            <IconButton title="Delete rule" icon="trash" onClick={() => onChange(rules.filter((_, i) => i !== index))} />
          </HStack>
        ))}
        <HStack gap={8}>
          <Button kind="secondary" onClick={() => onChange([...rules, { pattern: '', decision: 'ask', kind: null }])}>Add rule</Button>
        </HStack>
        <HStack gap={8} className="rule-try">
          <AutoTextArea className="field-box rule-pattern" value={command} placeholder="Try a command, e.g. git push origin main" minRows={1} maxRows={2}
            onChange={(value) => { setCommand(value); setAnswer(null) }} />
          <Button kind="secondary" disabled={!command.trim()} onClick={() => void run()}>Test</Button>
        </HStack>
        {answer ? <Text size="footnote" tone="secondary">{answer}</Text> : null}
      </VStack>
    </Field>
  )
}
