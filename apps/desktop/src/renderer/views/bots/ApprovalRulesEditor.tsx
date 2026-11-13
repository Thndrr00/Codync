import { useState } from 'react'
import { Button, ChoicePicker, IconButton } from '../../components/Controls'
import { Switch } from '../../components/Controls'
import { ModalHeader, Sheet } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import {
  CHAINING_WARNING,
  describeEvaluation,
  moveRule,
  QUICK_CLASSES,
  RULE_KINDS,
  type ApprovalEvaluation,
  type ApprovalRule,
  type QuickPolicy,
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
export function ApprovalRulesEditor({ rules, onChange, test, quick, onQuick }: {
  rules: ApprovalRule[]
  onChange: (rules: ApprovalRule[]) => void
  /** Asks the host what the bot would do with `command` under `rules`; null when it can't say. */
  test: (command: string, rules: ApprovalRule[]) => Promise<ApprovalEvaluation | null>
  /** The quick rules by kind of action; absent from a host that has none. */
  quick?: { on: boolean; policies: Partial<Record<string, QuickPolicy>> }
  onQuick?: (next: { on: boolean; policies: Partial<Record<string, QuickPolicy>> }) => void
}) {
  const [command, setCommand] = useState('')
  const [answer, setAnswer] = useState<string | null>(null)
  const [transferOpen, setTransferOpen] = useState(false)
  const [transferText, setTransferText] = useState('')
  const [transferError, setTransferError] = useState<string | null>(null)
  const replace = (index: number, patch: Partial<ApprovalRule>) =>
    onChange(rules.map((rule, i) => (i === index ? { ...rule, ...patch } : rule)))
  const run = async () => {
    if (!command.trim()) return
    const evaluation = await test(command, rules)
    setAnswer(evaluation ? describeEvaluation(evaluation) : "This computer's host can't test rules.")
  }
  const openTransfer = () => {
    setTransferText(JSON.stringify(rules, null, 2))
    setTransferError(null)
    setTransferOpen(true)
  }
  const applyTransfer = () => {
    try {
      const parsed: unknown = JSON.parse(transferText)
      if (!Array.isArray(parsed)) throw new Error('The rules are a JSON list.')
      const next: ApprovalRule[] = parsed.map((item, index) => {
        if (!item || typeof item !== 'object') throw new Error(`Rule ${index + 1} is not an object.`)
        const rule = item as Partial<ApprovalRule>
        if (!['allow', 'ask', 'deny'].includes(String(rule.decision))) throw new Error(`Rule ${index + 1} has an unknown decision.`)
        return { pattern: String(rule.pattern ?? ''), decision: rule.decision as RuleDecision, kind: rule.kind ?? null }
      })
      onChange(next)
      setTransferOpen(false)
    } catch (e) {
      setTransferError(e instanceof Error ? e.message : String(e))
    }
  }
  return (
    <>
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
          <Button kind="secondary" onClick={openTransfer}>Copy or import rules</Button>
        </HStack>
        {quick && onQuick ? (
          <VStack gap={6}>
            <HStack gap={8}>
              <Switch on={quick.on} onChange={(on) => onQuick({ ...quick, on })}>
                <Text weight="semibold">Quick rules by kind of action</Text>
              </Switch>
            </HStack>
            <Text size="footnote" tone="secondary">
              {quick.on ? 'After your rules above, these decide by the kind of tool. ' : 'Off: nothing below is enforced. '}
              Browser, apps, team and routine tools are governed by the connectors' tool policies, not here.
            </Text>
            {QUICK_CLASSES.map((c) => (
              <HStack key={c.id} gap={8}>
                <Text className="rule-quick-name">{c.label}</Text>
                <ChoicePicker
                  selection={quick.policies[c.id] ?? ''}
                  options={[{ id: '', label: 'Not set' }, { id: 'allow', label: 'Allow' }, { id: 'ask', label: 'Ask me' }, { id: 'block', label: 'Deny' }]}
                  onChange={(id) => onQuick({ ...quick, policies: { ...quick.policies, [c.id]: id || undefined } })}
                />
                <Text size="caption" tone="tertiary">{c.kinds}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        <HStack gap={8} className="rule-try">
          <AutoTextArea className="field-box rule-pattern" value={command} placeholder="Try a command, e.g. git push origin main" minRows={1} maxRows={2}
            onChange={(value) => { setCommand(value); setAnswer(null) }} />
          <Button kind="secondary" disabled={!command.trim()} onClick={() => void run()}>Test</Button>
        </HStack>
        {answer ? <Text size="footnote" tone="secondary">{answer}</Text> : null}
      </VStack>
    </Field>
    <Sheet open={transferOpen} onClose={() => setTransferOpen(false)} width={560} maxHeight="82vh">
      <VStack gap={12}>
        <ModalHeader icon="doc.on.doc" title="Rules as JSON" subtitle="Copy these to another bot, or paste another bot's rules and apply them." onClose={() => setTransferOpen(false)} />
        <Field label="Rules" value={transferText} onChange={setTransferText} lines={[10, 20]} mono />
        {transferError ? <Text size="caption" tone="danger">{transferError}</Text> : null}
        <HStack justify="end" gap={8}>
          <Button kind="secondary" onClick={() => void window.codync.app.copy(transferText)}>Copy</Button>
          <Button kind="secondary" onClick={() => setTransferOpen(false)}>Close</Button>
          <Button onClick={applyTransfer}>Apply</Button>
        </HStack>
      </VStack>
    </Sheet>
    </>
  )
}
