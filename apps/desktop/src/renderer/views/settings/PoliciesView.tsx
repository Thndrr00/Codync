import { useCallback, useEffect, useState } from 'react'
import { Button, CardForm, CardSection, ChoicePicker, IconButton, Switch, ValueRow } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import {
  blankPolicy,
  cleanPolicy,
  fromPreset,
  INITIATORS,
  policyProblem,
  variableRows,
  type PoliciesReply,
  type PolicyDraft,
  type PolicyPreview,
} from '@shared/policies'
import { AutoTextArea } from '../bots/parts'
import { fill } from '../../lib/fill.ts'
import { errorText } from './parts'

/**
 * This computer's policies: named CEL rules with an effect and a message, evaluated before any bot's
 * approval rules, plus a what-if that never acts. The host compiles and decides; this only words it
 * (`docs/features/policy-engine.md`).
 */
export function PoliciesView() {
  const app = useApp()
  const local = useModel(app.local)
  const tr = useTr()
  const [reply, setReply] = useState<PoliciesReply | null>(null)
  const [rows, setRows] = useState<PolicyDraft[]>([])
  const [problems, setProblems] = useState<Record<number, string>>({})
  const [busy, setBusy] = useState<number | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [tool, setTool] = useState('shell')
  const [command, setCommand] = useState('')
  const [initiator, setInitiator] = useState<(typeof INITIATORS)[number]['id']>('person')
  const [preview, setPreview] = useState<PolicyPreview | null>(null)
  const [asking, setAsking] = useState(false)

  const load = useCallback(async () => {
    if (!local) return
    try {
      const answer = await local.policies()
      setReply(answer)
      setRows(answer.policies.map((policy) => ({ ...policy })))
      setProblems({})
      setLoadError(null)
    } catch (e) {
      setLoadError(errorText(e))
    }
  }, [local])

  useEffect(() => {
    void load()
  }, [load])

  if (!local) {
    return (
      <CardForm>
        <Text tone="secondary">
          {tr("Policies are kept by this computer's host, which isn't running.")}
        </Text>
      </CardForm>
    )
  }
  if (!reply) {
    return <CardForm>{loadError ? <Text tone="danger">{loadError}</Text> : <Text tone="secondary">{tr('Reading the policies…')}</Text>}</CardForm>
  }

  const replace = (index: number, patch: Partial<PolicyDraft>) =>
    setRows((current) => current.map((row, i) => (i === index ? { ...row, ...patch } : row)))
  const complain = (index: number, message: string | null) =>
    setProblems((current) => {
      const next: Record<number, string | undefined> = { ...current }
      if (message) next[index] = message
      else delete next[index]
      return next
    })

  const save = async (index: number) => {
    const row = rows[index]
    const problem = policyProblem(row, reply.limits)
    complain(index, problem)
    if (problem) return
    setBusy(index)
    try {
      await local.savePolicy(cleanPolicy(row))
      await load()
    } catch (e) {
      complain(index, errorText(e))
    } finally {
      setBusy(null)
    }
  }

  const toggle = async (index: number) => {
    const row = rows[index]
    replace(index, { enabled: !row.enabled })
    if (!row.id) return
    setBusy(index)
    try {
      await local.setPolicyEnabled(row.id, !row.enabled)
      await load()
    } catch (e) {
      complain(index, errorText(e))
    } finally {
      setBusy(null)
    }
  }

  const remove = async (index: number) => {
    const row = rows[index]
    if (!row.id) {
      setRows((current) => current.filter((_, i) => i !== index))
      return
    }
    setBusy(index)
    try {
      await local.removePolicy(row.id)
      await load()
    } catch (e) {
      complain(index, errorText(e))
    } finally {
      setBusy(null)
    }
  }

  const ask = async () => {
    if (!tool.trim()) return
    setAsking(true)
    try {
      // The list as it stands, saved or not: what a rule would do is answered by the same engine.
      setPreview(
        await local.previewPolicy({
          policies: rows.map(cleanPolicy),
          tool: tool.trim(),
          ...(command.trim() ? { command } : {}),
          initiator,
        }),
      )
    } catch (e) {
      setPreview({ decision: 'deny', source: 'error', rule: null, allowlist: false, refusal: null, variables: {}, explanation: errorText(e) })
    } finally {
      setAsking(false)
    }
  }

  return (
    <CardForm>
      <CardSection
        title={tr('Policies')}
        footer={tr(
          "A deny rule refuses first. An allow rule permits, and once one is saved only what an allow rule names goes through; without policies the bots' approval rules and Approvals setting decide as before. A rule that cannot run refuses and says why.",
        )}
      >
        <VStack gap={10}>
          {rows.length === 0 ? (
            <Text size="footnote" tone="secondary">
              {tr('No policies: start from a preset, or add a rule of your own.')}
            </Text>
          ) : null}
          {rows.map((row, index) => (
            <VStack key={row.id ?? `new-${index}`} gap={6} className="rule-row">
              <HStack gap={8} align="center">
                <Switch on={row.enabled} onChange={() => void toggle(index)} label={tr('On')} />
                <ChoicePicker
                  selection={row.effect}
                  options={[
                    { id: 'deny', label: tr('Deny') },
                    { id: 'allow', label: tr('Allow') },
                  ]}
                  onChange={(effect) => replace(index, { effect: effect as PolicyDraft['effect'] })}
                />
                <AutoTextArea
                  className="field-box rule-pattern"
                  value={row.name}
                  placeholder={tr('Rule name')}
                  minRows={1}
                  maxRows={2}
                  onChange={(name) => replace(index, { name })}
                />
                <IconButton title={tr('Delete rule')} icon="trash" disabled={busy === index} onClick={() => void remove(index)} />
              </HStack>
              <AutoTextArea
                className="field-box"
                value={row.expression}
                placeholder={'tool.kind == "execute"'}
                minRows={1}
                maxRows={4}
                onChange={(expression) => replace(index, { expression })}
              />
              <HStack gap={8} align="center">
                <AutoTextArea
                  className="field-box"
                  value={row.message}
                  placeholder={tr('What a person reads when this rule decides')}
                  minRows={1}
                  maxRows={2}
                  onChange={(message) => replace(index, { message })}
                />
                <Button kind="secondary" disabled={busy === index} onClick={() => void save(index)}>
                  {row.id ? tr('Save') : tr('Add rule')}
                </Button>
              </HStack>
              {problems[index] ? (
                <Text size="footnote" tone="danger">
                  {problems[index]}
                </Text>
              ) : null}
            </VStack>
          ))}
          <HStack gap={8} wrap>
            <Button kind="secondary" onClick={() => setRows((current) => [...current, blankPolicy()])}>
              {tr('New rule')}
            </Button>
            {reply.presets.map((preset) => (
              <Button
                key={preset.id}
                kind="secondary"
                title={preset.about}
                onClick={() => setRows((current) => [...current, fromPreset(preset)])}
              >
                {fill(tr('Start from “{name}”'), { name: preset.name })}
              </Button>
            ))}
          </HStack>
          <Text size="caption" tone="secondary">
            {reply.enforcing
              ? reply.allowlist
                ? tr('Policies are on. An allow rule is saved, so only what an allow rule names goes through.')
                : tr("Policies are on. What they do not refuse goes on to the bot's approval rules.")
              : tr("No policy is on: the bots' approval rules and Approvals setting decide everything, as before.")}
          </Text>
        </VStack>
      </CardSection>

      <CardSection
        title={tr('What would happen?')}
        footer={tr('Answered by the same engine a real request goes through, with credentials masked. Nothing runs.')}
      >
        <ValueRow label={tr('Tool')}>
          <AutoTextArea className="field-box rule-pattern" value={tool} minRows={1} maxRows={2} placeholder="shell" onChange={setTool} />
        </ValueRow>
        <ValueRow label={tr('Command or input')}>
          <AutoTextArea
            className="field-box rule-pattern"
            value={command}
            minRows={1}
            maxRows={2}
            placeholder={tr('What it would be asked to do')}
            onChange={(value) => {
              setCommand(value)
              setPreview(null)
            }}
          />
        </ValueRow>
        <ValueRow label={tr('Who asks')}>
          <ChoicePicker
            selection={initiator}
            options={INITIATORS.map((item) => ({ id: item.id, label: tr(item.label) }))}
            onChange={(id) => setInitiator(id as (typeof INITIATORS)[number]['id'])}
          />
        </ValueRow>
        <HStack gap={8}>
          <Button kind="secondary" disabled={asking || !tool.trim()} onClick={() => void ask()}>
            {asking ? tr('Asking…') : tr('Ask')}
          </Button>
        </HStack>
        {preview ? (
          <VStack gap={6}>
            <Text weight="semibold">
              {preview.decision === 'allow'
                ? tr('Allowed')
                : preview.decision === 'deny'
                  ? preview.rule
                    ? fill(tr('Refused by “{name}”'), { name: preview.rule.name })
                    : tr('Refused')
                  : tr("No policy decides: the bot's approval rules do")}
            </Text>
            <Text size="footnote" tone="secondary">
              {preview.explanation}
            </Text>
            {preview.rules?.length ? (
              <VStack gap={2}>
                {preview.rules.map((rule) => (
                  <HStack key={`${rule.index}-${rule.id}`} gap={8}>
                    <Text size="caption" tone="tertiary">
                      {rule.effect === 'allow' ? tr('Allow') : tr('Deny')}
                    </Text>
                    <Text size="caption" tone={rule.matched ? 'text' : 'secondary'} truncate>
                      {rule.name}
                    </Text>
                    <Text size="caption" tone="tertiary">
                      {rule.error ?? (rule.matched ? tr('matches') : tr('does not match'))}
                    </Text>
                  </HStack>
                ))}
              </VStack>
            ) : null}
            <VStack gap={2}>
              {variableRows(preview.variables).map((variable) => (
                <HStack key={variable.name} gap={8}>
                  <Text size="caption" tone="tertiary" mono>
                    {variable.name}
                  </Text>
                  <Text size="caption" tone="secondary" truncate>
                    {variable.value || '—'}
                  </Text>
                </HStack>
              ))}
            </VStack>
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Variables a rule can use')} footer={tr('Every value is a string; an empty one means the call has none.')}>
        {reply.variables.map((variable) => (
          <ValueRow key={variable.name} label={variable.name} detail={variable.about}>
            <Text size="caption" tone="tertiary" mono>
              {variable.name}
            </Text>
          </ValueRow>
        ))}
      </CardSection>
    </CardForm>
  )
}
