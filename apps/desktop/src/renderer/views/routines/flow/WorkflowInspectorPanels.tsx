import { useEffect, useState, type ReactNode } from 'react'
import { Button, ChoicePicker, Switch } from '../../../components/Controls'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import {
  DELIVERY_OUTCOME_LABEL, SKIP_REASON_LABEL, WORKFLOW_GITHUB_ACTIONS, WORKFLOW_GITHUB_EVENTS,
  WORKFLOW_GITHUB_FILTERS, WORKFLOW_IMAGE_INPUTS, WORKFLOW_OUTPUT_TARGETS, WORKFLOW_TRIGGER_TYPES,
  labelOf, nodeEditable, triggerFields, type FlowField, type WorkflowFields, type WorkflowGitHubHistory,
  type WorkflowGraph, type WorkflowHistoryStep, type WorkflowNode, type WorkflowTriggerState,
} from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { FIELD_TYPE_LABEL } from './FieldPicker'
import { FieldTextArea } from './FieldTextArea'
import { StepFieldsToolbar } from './FlowInspector'

const valueText = (fields: WorkflowFields, name: string): string => {
  const value = fields[name]
  return value === undefined || value === null ? '' : String(value)
}

function FieldRow({ label, children }: { label: string; children: ReactNode }) {
  const tr = useTr()
  return (
    <VStack gap={4}>
      <Text size="caption" tone="secondary">{tr(label)}</Text>
      {children}
    </VStack>
  )
}

function TextField({ label, value, onChange, readOnly, placeholder }: {
  label: string; value: string; onChange: (value: string) => void; readOnly: boolean; placeholder?: string
}) {
  const tr = useTr()
  return (
    <FieldRow label={label}>
      <input
        className="routine-input"
        aria-label={tr(label)}
        placeholder={placeholder ? tr(placeholder) : undefined}
        value={value}
        disabled={readOnly}
        onChange={(event) => onChange(event.target.value)}
      />
    </FieldRow>
  )
}

function ChoiceField<T extends string>({ label, value, options, onChange, readOnly }: {
  label: string; value: string; options: readonly T[]; onChange: (value: string) => void; readOnly: boolean
}) {
  return (
    <FieldRow label={label}>
      <ChoicePicker<string>
        selection={value || options[0] || ''}
        options={options.map((option) => ({ id: option, label: option }))}
        onChange={(next) => { if (!readOnly) onChange(next) }}
      />
    </FieldRow>
  )
}

function NumberField({ label, value, onChange, readOnly }: {
  label: string; value: number | undefined; onChange: (value: number) => void; readOnly: boolean
}) {
  const tr = useTr()
  return (
    <FieldRow label={label}>
      <input
        className="routine-input"
        type="number"
        aria-label={tr(label)}
        value={value ?? ''}
        disabled={readOnly}
        onChange={(event) => { const next = Number(event.target.value); if (Number.isFinite(next)) onChange(next) }}
      />
    </FieldRow>
  )
}

/** The saved trigger fields, as typed chips the person can copy into a later instruction. */
function TriggerFields({ kind }: { kind: string }) {
  const tr = useTr()
  const fields = triggerFields(kind)
  if (fields.length === 0) return null
  return (
    <VStack gap={4}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Fields this trigger gives')}</Text>
      <HStack gap={4} wrap>
        {fields.map((field: FlowField) => (
          <span key={field.name} className="flow-trigger-field">{field.name}
            <Text size="caption" tone="secondary">{tr(FIELD_TYPE_LABEL[field.type])}</Text>
          </span>
        ))}
      </HStack>
    </VStack>
  )
}

/** Delivered events with the reason a run did not start, and a way to replay the last one. */
function DeliveryHistory({ history, onRunLastEvent }: { history: WorkflowGitHubHistory | null; onRunLastEvent: () => void }) {
  const tr = useTr()
  if (!history || history.deliveries.length === 0) {
    return <Text size="caption" tone="secondary">{tr('No deliveries yet.')}</Text>
  }
  return (
    <VStack gap={6}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Delivery history')}</Text>
      {history.deliveries.slice(0, 20).map((delivery) => (
        <HStack key={delivery.id} gap={8} align="center" className="flow-delivery">
          <Text size="caption" truncate grow>{delivery.event}</Text>
          <Text size="caption" tone="secondary">
            {tr(delivery.outcome === 'skipped' && delivery.reason
              ? `${labelOf(DELIVERY_OUTCOME_LABEL, delivery.outcome, tr)} · ${labelOf(SKIP_REASON_LABEL, delivery.reason, tr)}`
              : labelOf(DELIVERY_OUTCOME_LABEL, delivery.outcome, tr))}
          </Text>
        </HStack>
      ))}
      <Button kind="secondary" onClick={onRunLastEvent}>{tr('Run with last event')}</Button>
    </VStack>
  )
}

export interface WorkflowPanelProps {
  node: WorkflowNode
  graph: WorkflowGraph
  triggers: WorkflowTriggerState[]
  steps: Record<string, WorkflowHistoryStep | null>
  readOnly: boolean
  fieldsFor: (nodeKey: string) => FlowField[]
  problemsFor: (nodeKey: string) => Record<string, string>
  answerFor: (nodeKey: string) => string
  checkFor: (nodeKey: string) => string
  onSaveAnswer: (nodeKey: string, text: string) => void
  onSaveCheck: (nodeKey: string, text: string) => void
  onSaveFields: (node: WorkflowNode, fields: WorkflowFields) => void
  onDecide: (stepId: string, approved: boolean) => void
  onRotateSecret: (nodeId: string) => void
  onRunLastEvent: (nodeId: string) => void
  onLoadTriggerHistory: (nodeId: string) => void
  triggerHistory: WorkflowGitHubHistory | null
  secret: string | null
}

/**
 * The inspector of a portable node. Everything the host owns is edited through `onSaveFields` with
 * the revision of the graph it came from; a locked or unknown node has no inputs at all. Shared
 * controls (ChoicePicker, Switch, FieldTextArea) are the same ones the routine steps use.
 */
export function WorkflowNodePanel(props: WorkflowPanelProps) {
  const { node, graph, triggers, steps, readOnly, fieldsFor, problemsFor, answerFor, checkFor,
    onSaveAnswer, onSaveCheck, onSaveFields, onDecide, onRotateSecret, onRunLastEvent, onLoadTriggerHistory,
    triggerHistory, secret } = props
  const tr = useTr()
  const nodeKey = `workflow:${node.id}`
  const locked = readOnly || !nodeEditable(node)
  const [draft, setDraft] = useState<WorkflowFields>({ ...node.fields })
  const kept = JSON.stringify(node.fields)
  useEffect(() => setDraft({ ...node.fields }), [node.id, kept])
  const set = (name: string, value: WorkflowFields[string]) => setDraft((current) => ({ ...current, [name]: value }))
  const step = steps[node.id] ?? null
  const changed = kept !== JSON.stringify(draft)
  const trigger = node.kind === 'trigger' ? triggers.find((item) => item.nodeId === node.id) ?? null : null
  const triggerKind = valueText(draft, 'trigger') || trigger?.type || ''
  useEffect(() => {
    if (node.kind === 'trigger' && ['github', 'discord', 'telegram', 'slack'].includes(triggerKind)) onLoadTriggerHistory(node.id)
    // The host delivers events while this panel is open; reload on the node and trigger kind.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.id, triggerKind])

  if (locked) {
    return (
      <VStack gap={10} className="flow-inspector">
        <Text weight="semibold">{tr('Read-only node')}</Text>
        <Text size="callout" tone="secondary">
          {tr(nodeEditable(node) ? 'This workflow uses a newer format. Update Codync to change it.'
            : 'This step was added by a newer version of Codync.')}
        </Text>
        <span className="flow-newer-badge">{tr('Needs newer app')}</span>
      </VStack>
    )
  }

  const prompt = (name: string, label: string, placeholder: string) => (
    <FieldTextArea
      value={valueText(draft, name)}
      onChange={(text) => set(name, text)}
      fields={fieldsFor(nodeKey)}
      problems={problemsFor(nodeKey)}
      disabled={locked}
      label={label}
      placeholder={placeholder}
    />
  )

  return (
    <VStack gap={10} className="flow-inspector flow-workflow-inspector">
      <Text weight="semibold">{tr(node.kind)}</Text>
      {graph.nodes.length === 0 ? null : <Text size="caption" tone="secondary">{node.id}</Text>}
      {node.kind === 'llm' ? (
        <>
          <TextField label="Provider" value={valueText(draft, 'provider')} onChange={(value) => set('provider', value)} readOnly={locked} />
          <TextField label="Model" value={valueText(draft, 'model')} onChange={(value) => set('model', value)} readOnly={locked} />
          <ChoiceField label="Effort" value={valueText(draft, 'effort')} options={['low', 'medium', 'high']} onChange={(value) => set('effort', value)} readOnly={locked} />
          {prompt('system', 'System', 'Instructions for the model')}
          {prompt('prompt', 'Prompt', 'Type your prompt here')}
          <StepFieldsToolbar nodeKey={nodeKey} disabled={locked} answerFor={answerFor} checkFor={checkFor}
            onSaveAnswer={onSaveAnswer} onSaveCheck={onSaveCheck} />
        </>
      ) : null}
      {node.kind === 'condition' ? (
        <>
          {prompt('rule', 'Rule', 'What decides the branch?')}
          {prompt('check', 'Check', 'The condition to test')}
          <Text size="caption" tone="secondary">{tr('Two lines leave this step: Yes and No. Connect each one to the step it should start.')}</Text>
        </>
      ) : null}
      {node.kind === 'approval' ? (
        <>
          {prompt('question', 'Question', 'What should the person decide?')}
          <HStack gap={8}>
            <Button disabled={!step?.waiting} onClick={() => step && onDecide(step.id, true)}>{tr('Approve')}</Button>
            <Button kind="secondary" disabled={!step?.waiting} onClick={() => step && onDecide(step.id, false)}>{tr('Reject')}</Button>
            <Spacer />
            {!step?.waiting ? <Text size="caption" tone="secondary">{tr('Not waiting')}</Text> : null}
          </HStack>
        </>
      ) : null}
      {node.kind === 'output' ? (
        <>
          <ChoiceField label="Target" value={valueText(draft, 'target')} options={WORKFLOW_OUTPUT_TARGETS} onChange={(value) => set('target', value)} readOnly={locked} />
          <ChoiceField label="Action" value={valueText(draft, 'action')} options={WORKFLOW_GITHUB_ACTIONS} onChange={(value) => set('action', value)} readOnly={locked} />
          <TextField label="Connection" value={valueText(draft, 'connectionId')} onChange={(value) => set('connectionId', value)} readOnly={locked} placeholder="Connection id" />
          <TextField label="Channel" value={valueText(draft, 'channelId')} onChange={(value) => set('channelId', value)} readOnly={locked} placeholder="Channel id" />
          {prompt('message', 'Message', 'What to send')}
          <TextField label="Title" value={valueText(draft, 'title')} onChange={(value) => set('title', value)} readOnly={locked} />
          <TextField label="Labels" value={valueText(draft, 'labels')} onChange={(value) => set('labels', value)} readOnly={locked} />
          <TextField label="Person" value={valueText(draft, 'person')} onChange={(value) => set('person', value)} readOnly={locked} />
          <ChoiceField label="Close reason" value={valueText(draft, 'reason')} options={['completed', 'notPlanned']} onChange={(value) => set('reason', value)} readOnly={locked} />
        </>
      ) : null}
      {node.kind === 'asset' ? (
        <TextField label="File" value={valueText(draft, 'fileId')} onChange={(value) => set('fileId', value)} readOnly={locked} placeholder="File id on this computer" />
      ) : null}
      {node.kind === 'trigger' ? (
        <>
          <ChoiceField label="Trigger" value={triggerKind} options={WORKFLOW_TRIGGER_TYPES} onChange={(value) => set('trigger', value)} readOnly={locked} />
          <TextField label="Connection" value={valueText(draft, 'connectionId')} onChange={(value) => set('connectionId', value)} readOnly={locked} placeholder="Connection id" />
          <TextField label="Place" value={valueText(draft, 'placeId')} onChange={(value) => set('placeId', value)} readOnly={locked} placeholder="Place or server id" />
          <TextField label="Channel" value={valueText(draft, 'channelId')} onChange={(value) => set('channelId', value)} readOnly={locked} placeholder="Channel id" />
          <TextField label="Roles" value={valueText(draft, 'roles')} onChange={(value) => set('roles', value)} readOnly={locked} placeholder="Who may trigger this" />
          <TextField label="Repositories" value={valueText(draft, 'repositories')} onChange={(value) => set('repositories', value)} readOnly={locked} placeholder="owner/repo, one per line" />
          {triggerKind === 'github' ? (
            <>
              <ChoiceField label="Event" value={valueText(draft, 'event')} options={WORKFLOW_GITHUB_EVENTS} onChange={(value) => set('event', value)} readOnly={locked} />
              <FieldRow label="Filters">
                {WORKFLOW_GITHUB_FILTERS.map((filter) => (
                  <Switch
                    key={filter}
                    on={valueText(draft, 'filters').split(/[\n,]/).map((item) => item.trim()).includes(filter)}
                    label={filter}
                    disabled={locked}
                    onChange={(on) => {
                      const chosen = new Set(valueText(draft, 'filters').split(/[\n,]/).map((item) => item.trim()).filter(Boolean))
                      if (on) chosen.add(filter)
                      else chosen.delete(filter)
                      set('filters', [...chosen].join('\n'))
                    }}
                  />
                ))}
              </FieldRow>
              <TextField label="Branch" value={valueText(draft, 'branch')} onChange={(value) => set('branch', value)} readOnly={locked} />
              <TextField label="Title text" value={valueText(draft, 'titleText')} onChange={(value) => set('titleText', value)} readOnly={locked} />
            </>
          ) : null}
          {['discord', 'telegram', 'slack'].includes(triggerKind) ? (
            <>
              <TextField label="Text to watch for" value={valueText(draft, 'text')} onChange={(value) => set('text', value)} readOnly={locked} />
              {prompt('answer', 'Answer', 'What to answer when it matches')}
            </>
          ) : null}
          {trigger?.webhookUrl ? (
            <VStack gap={4}>
              <Text size="caption" tone="secondary">{tr('Webhook URL')}</Text>
              <Text size="callout" className="flow-webhook-url">{trigger.webhookUrl}</Text>
              <Button kind="secondary" disabled={locked} onClick={() => onRotateSecret(node.id)}>{tr('Rotate the secret')}</Button>
              {secret ? <Text size="caption" tone="secondary" className="flow-webhook-secret">{tr('Secret')}: {secret}</Text> : null}
            </VStack>
          ) : null}
          {trigger?.nextDueAt ? (
            <Text size="caption" tone="secondary">{tr('Next run')}: {new Date(trigger.nextDueAt).toLocaleString()}</Text>
          ) : null}
          <TriggerFields kind={triggerKind} />
          <DeliveryHistory history={triggerHistory} onRunLastEvent={() => onRunLastEvent(node.id)} />
        </>
      ) : null}
      {node.kind === 'image' ? (
        <>
          {prompt('prompt', 'What to make', 'Describe the picture or clip')}
          <ChoiceField label="Media" value={valueText(draft, 'media')} options={['image', 'video']} onChange={(value) => set('media', value)} readOnly={locked} />
          <TextField label="Model" value={valueText(draft, 'model')} onChange={(value) => set('model', value)} readOnly={locked} />
          <NumberField label="Custom width" value={typeof draft.width === 'number' ? draft.width : undefined} onChange={(value) => set('width', value)} readOnly={locked} />
          <NumberField label="Custom height" value={typeof draft.height === 'number' ? draft.height : undefined} onChange={(value) => set('height', value)} readOnly={locked} />
          <ChoiceField label="Background" value={valueText(draft, 'background')} options={['solid', 'transparent']} onChange={(value) => set('background', value)} readOnly={locked} />
          <TextField label="First frame from step" value={valueText(draft, 'firstFrame')} onChange={(value) => set('firstFrame', value)} readOnly={locked} />
          <TextField label="Last frame from step" value={valueText(draft, 'lastFrame')} onChange={(value) => set('lastFrame', value)} readOnly={locked} />
          <ChoiceField label="Mode" value={valueText(draft, 'mode')} options={WORKFLOW_IMAGE_INPUTS} onChange={(value) => set('mode', value)} readOnly={locked} />
          <TextField label="Provider options" value={valueText(draft, 'options')} onChange={(value) => set('options', value)} readOnly={locked} placeholder="JSON of provider options" />
        </>
      ) : null}
      {(node.kind === 'text' || node.kind === 'note' || node.kind === 'shape') ? (
        <>
          {prompt('text', 'Text', 'What the note says')}
          <FieldRow label="Font size">
            <ChoicePicker<number>
              selection={node.size?.font ?? 14}
              options={[11, 14, 18, 24, 32].map((size) => ({ id: size, label: `${size}` }))}
              onChange={(font) => onSaveFields({ ...node, size: { ...node.size, font } }, draft)}
            />
          </FieldRow>
          <NumberField label="Width" value={node.size?.width} onChange={(width) => onSaveFields({ ...node, size: { ...node.size, width } }, draft)} readOnly={locked} />
          <NumberField label="Height" value={node.size?.height} onChange={(height) => onSaveFields({ ...node, size: { ...node.size, height } }, draft)} readOnly={locked} />
        </>
      ) : null}
      {node.kind === 'prompt' || node.kind === 'agent' ? prompt('task', 'Task', 'Type your prompt here') : null}
      <HStack gap={8}>
        <Button disabled={locked || !changed} onClick={() => onSaveFields(node, draft)}>{tr('Save')}</Button>
      </HStack>
    </VStack>
  )
}
