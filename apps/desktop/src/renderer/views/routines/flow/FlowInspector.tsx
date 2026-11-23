import { useEffect, useRef, useState } from 'react'
import { onModel, type MediaAnswer, type MediaKind, type MediaModel } from '@shared/media'
import type { Bot } from '@shared/models'
import { Button, ChipChoice, DropdownMenu, Switch } from '../../../components/Controls'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'
import {
  botKey, latestStep, mediaKey, mediaTitle, statusLabel, stepNameOf, stepOf,
  type FlowCanvas, type FlowField, type FlowLink, type FlowMedia, type FlowRoutineCard, type FlowStep,
} from '../../../lib/routine-flow.ts'
import { useStore } from '../../../store/context'
import { useAgentModels } from '../../bots/agent-models'
import { AgentModelPicker } from '../../bots/AgentModelPicker'
import { runLabel } from '../routine'
import { AnswerFieldsPanel, CheckPanel } from './AnswerFields'
import type { FlowSelection } from './FlowCanvas'
import { FieldTextArea } from './FieldTextArea'
import { FlowPopover } from './FlowPopover'
import { groupsOf, type FlowGroup } from './flow-groups'
import { StatusChip } from './FlowNodeCard'
import { StepResult } from './StepResult'

/** A step, bot or media node the selected step may hand work to. */
export interface FlowTarget {
  id: string
  name: string
}

/** The settings a media step is saved with. */
export interface MediaSettings {
  model: string
  prompt: string
  aspect: string
  duration: number
  resolution: string
  transparent: boolean
  voice: string | null
}

/** The settings a media step is edited with, plus the kind its model makes. */
type MediaDraft = MediaSettings & { kind: MediaKind }

const mediaDraft = (media: FlowMedia): MediaDraft => ({
  kind: media.kind,
  model: media.model,
  prompt: media.prompt,
  aspect: media.aspect,
  duration: media.duration,
  resolution: media.resolution,
  transparent: media.transparent,
  voice: media.voice,
})

interface Props {
  canvas: FlowCanvas
  bots: ReadonlyMap<string, Bot>
  selection: FlowSelection
  busy: boolean
  offline: boolean
  /** A setting from a newer version is on this workflow: none of its steps may be saved. */
  readOnly: boolean
  /** Fields earlier steps (and the trigger) give this step, for `{{` completion and Insert field. */
  fieldsFor: (nodeKey: string) => FlowField[]
  /** Problems with those fields, by name, shown in the warning chip and on hover. */
  problemsFor: (nodeKey: string) => Record<string, string>
  /** The step's saved "name: type" answer text and check, edited from the toolbar only. */
  answerFor: (nodeKey: string) => string
  checkFor: (nodeKey: string) => string
  onSaveAnswer: (nodeKey: string, text: string) => void
  onSaveCheck: (nodeKey: string, text: string) => void
  onSaveInstruction: (link: FlowLink, instruction: string, model: string | null, modelOptions: Record<string, string>) => void
  onDisconnect: (link: FlowLink) => void
  onTest: (routineId: string) => void
  onSetEnabled: (routineId: string, enabled: boolean) => void
  onConnectFrom: (fromBotId: string, toBotId: string) => void
  onRemoveNode: (nodeKey: string) => void
  onAddMedia: (routineId: string, kind: MediaKind) => void
  onSaveMedia: (media: FlowMedia, settings: MediaSettings) => void
  onRemoveMedia: (media: FlowMedia) => void
  /** Shows the settings and inputs a step actually ran with. */
  onShowSettings: (card: FlowRoutineCard, step: FlowStep) => void
  /** Duplicates a picture step with its settings and connected reference pictures. */
  onDuplicateMedia: (media: FlowMedia) => void
  onRenameGroup: (groupId: string, name: string) => void
  onToggleGroup: (groupId: string) => void
  onUngroup: (groupId: string) => void
  /** The steps and bots `from` may hand work to, in some routine of this canvas. */
  targets: (fromBotId: string) => FlowTarget[]
}

/** The step toolbar's Answer fields and Check buttons; the prompt below stays a single text box. */
function StepFieldsToolbar({ nodeKey, disabled, answerFor, checkFor, onSaveAnswer, onSaveCheck }: Pick<Props,
  'answerFor' | 'checkFor' | 'onSaveAnswer' | 'onSaveCheck'> & { nodeKey: string; disabled: boolean }) {
  const tr = useTr()
  const answerButton = useRef<HTMLButtonElement>(null)
  const checkButton = useRef<HTMLButtonElement>(null)
  const [showAnswer, setShowAnswer] = useState(false)
  const [showCheck, setShowCheck] = useState(false)
  return (
    <HStack gap={6}>
      <button
        ref={answerButton}
        type="button"
        className="flow-menu-button"
        aria-haspopup="dialog"
        aria-expanded={showAnswer}
        disabled={disabled}
        onClick={() => setShowAnswer(!showAnswer)}
      >
        <Text size="caption">{tr('Answer fields')}</Text>
      </button>
      <button
        ref={checkButton}
        type="button"
        className="flow-menu-button"
        aria-haspopup="dialog"
        aria-expanded={showCheck}
        disabled={disabled}
        onClick={() => setShowCheck(!showCheck)}
      >
        <Text size="caption">{tr('Check')}</Text>
      </button>
      <FlowPopover open={showAnswer} onClose={() => setShowAnswer(false)} anchor={answerButton} title="Answer fields">
        <AnswerFieldsPanel
          text={answerFor(nodeKey)}
          onSave={(text) => { onSaveAnswer(nodeKey, text); setShowAnswer(false) }}
        />
      </FlowPopover>
      <FlowPopover open={showCheck} onClose={() => setShowCheck(false)} anchor={checkButton} title="Check">
        <CheckPanel text={checkFor(nodeKey)} onSave={(text) => { onSaveCheck(nodeKey, text); setShowCheck(false) }} />
      </FlowPopover>
    </HStack>
  )
}

/** The side panel: what the selected routine, bot, media step or link says, and the actions on it. */
export function FlowInspector(props: Props) {
  const { canvas, bots, selection } = props
  const tr = useTr()
  if (!selection) {
    return (
      <VStack gap={6} className="flow-inspector">
        <Text weight="semibold">{tr('Routine flow')}</Text>
        <Text size="callout" tone="secondary">{tr('Select a routine, a bot or a link. Drag from a bot\'s dot to another bot to hand its reply on.')}</Text>
      </VStack>
    )
  }
  if (selection.kind === 'edge') {
    const link = canvas.links.find((l) => l.id === selection.id)
    return link ? <LinkPanel {...props} link={link} bots={bots} /> : null
  }
  if (selection.kind === 'group') {
    const group = groupsOf(canvas).find((g) => g.id === selection.id)
    return group ? <GroupPanel {...props} group={group} /> : null
  }
  const [kind, id] = selection.key.split(':')
  if (kind === 'routine') {
    const card = canvas.routines.find((c) => c.routine.id === id)
    return card ? <RoutinePanel {...props} card={card} /> : null
  }
  if (kind === 'media') {
    const media = canvas.mediaNodes.find((m) => m.id === id)
    return media ? <MediaPanel {...props} media={media} /> : null
  }
  const bot = bots.get(id)
  return <BotPanel {...props} botId={id} bot={bot} />
}

/** A group of nodes: a name, its collapse state and the gesture that ends the group. */
function GroupPanel({ group, busy, offline, readOnly, onRenameGroup, onToggleGroup, onUngroup }: Props & { group: FlowGroup }) {
  const tr = useTr()
  const [name, setName] = useState(group.name)
  useEffect(() => setName(group.name), [group.id, group.name])
  const changed = name.trim() !== group.name
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{tr('Group')}</Text>
      <Text size="callout" tone="secondary">{tr('{count} nodes drawn together').replace('{count}', String(group.members.length))}</Text>
      <input
        className="routine-input flow-group-name"
        aria-label={tr('Group name')}
        placeholder={tr('Group name')}
        value={name}
        maxLength={120}
        disabled={offline || readOnly}
        onChange={(event) => setName(event.target.value)}
      />
      <Button disabled={busy || offline || readOnly || !changed} onClick={() => onRenameGroup(group.id, name.trim())}>{tr('Save name')}</Button>
      <Button kind="secondary" disabled={busy || offline || readOnly} onClick={() => onToggleGroup(group.id)}>{tr(group.collapsed ? 'Open group' : 'Collapse group')}</Button>
      <Button kind="secondary" disabled={busy || offline || readOnly} onClick={() => onUngroup(group.id)}>{tr('Ungroup')}</Button>
    </VStack>
  )
}

function LinkPanel({ link, bots, busy, offline, readOnly, fieldsFor, problemsFor, answerFor, checkFor, onSaveAnswer, onSaveCheck, onSaveInstruction, onDisconnect }: Props & { link: FlowLink }) {
  const tr = useTr()
  const nodeKey = botKey(link.toBotId)
  const fields = fieldsFor(nodeKey)
  const problems = problemsFor(nodeKey)
  const locked = busy || offline || readOnly
  const [text, setText] = useState(link.instruction)
  useEffect(() => setText(link.instruction), [link.id, link.instruction])
  // Saved with the button, like the instruction; the step keeps them for its one turn.
  const [model, setModel] = useState(link.model ?? null)
  useEffect(() => setModel(link.model ?? null), [link.id, link.model])
  const [modelOptions, setModelOptions] = useState<Record<string, string>>(link.modelOptions ?? {})
  useEffect(() => setModelOptions(link.modelOptions ?? {}), [link.id, link.modelOptions])
  const target = bots.get(link.toBotId)
  const models = useAgentModels(target?.backend ?? '', target?.account ?? null)
  const name = (id: string) => bots.get(id)?.name ?? tr('Unknown bot')
  const changed = text.trim() !== link.instruction || model !== (link.model ?? null)
    || JSON.stringify(modelOptions) !== JSON.stringify(link.modelOptions ?? {})
  const setModelChoice = (chosen: string | null) => {
    setModel(chosen)
    setModelOptions({})
  }
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{name(link.fromBotId)} → {name(link.toBotId)}</Text>
      <Text size="caption" tone="secondary">{tr('What the next bot is asked to do with the reply. It can be empty.')}</Text>
      <FieldTextArea
        value={text}
        onChange={setText}
        fields={fields}
        problems={problems}
        disabled={locked}
        label="Instruction"
        placeholder={tr('Type your prompt here')}
      />
      <StepFieldsToolbar
        nodeKey={nodeKey}
        disabled={locked}
        answerFor={answerFor}
        checkFor={checkFor}
        onSaveAnswer={onSaveAnswer}
        onSaveCheck={onSaveCheck}
      />
      <VStack gap={4}>
        <Text size="caption" tone="secondary">{tr('Model')}</Text>
        <Text size="caption" tone="secondary">{tr("The model this step runs on. Default is the bot's own.")}</Text>
        <AgentModelPicker backend={target?.backend ?? ''} models={models} selection={model} onChange={setModelChoice}
          options={modelOptions} onOptionsChange={setModelOptions} sessionNote={false} />
      </VStack>
      <HStack gap={8}>
        <Button disabled={locked || !changed} onClick={() => onSaveInstruction(link, text.trim(), model, modelOptions)}>{tr('Save')}</Button>
        <Spacer />
        <Button kind="secondary" disabled={locked} onClick={() => onDisconnect(link)}>{tr('Disconnect')}</Button>
      </HStack>
    </VStack>
  )
}

function RoutinePanel({ card, canvas, busy, offline, readOnly, onTest, onSetEnabled, onAddMedia, onShowSettings }: Props & { card: FlowRoutineCard }) {
  const tr = useTr()
  const { routine } = card
  const last = card.recentRuns[0]
  const linked = canvas.links.some((l) => l.routineId === routine.id)
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{routine.name}</Text>
      <Text size="callout" tone="secondary">{routine.triggerDescriptions.join(' · ')}</Text>
      <HStack gap={8}>
        <Text size="callout">{routine.enabled ? tr('Active') : tr('Paused')}</Text>
        <Spacer />
        <Switch on={routine.enabled} disabled={busy || offline || readOnly} label={routine.name} onChange={(on) => onSetEnabled(routine.id, on)} />
      </HStack>
      {routine.nextRunAt ? <Text size="caption" tone="secondary">{tr('Next run')}: {new Date(routine.nextRunAt).toLocaleString()}</Text> : null}
      <Text size="caption" tone="secondary">{last ? `${tr('Last run')}: ${tr(runLabel(last))}` : tr('No runs yet')}</Text>
      {linked ? null : <Text size="caption" tone="secondary">{tr('Not connected')}</Text>}
      <Button disabled={busy || offline || !linked} onClick={() => onTest(routine.id)}>{tr('Test the flow')}</Button>
      <HStack gap={8}>
        <Button kind="secondary" disabled={busy || offline || readOnly} onClick={() => onAddMedia(routine.id, 'image')}>{tr('Add a picture step')}</Button>
        <Button kind="secondary" disabled={busy || offline || readOnly} onClick={() => onAddMedia(routine.id, 'video')}>{tr('Add a clip step')}</Button>
      </HStack>
      <Steps canvas={canvas} card={card} onShowSettings={onShowSettings} />
      <History card={card} />
    </VStack>
  )
}

/** The steps of the latest run, one line each, with a bot's error or the start of its answer. */
function Steps({ canvas, card, onShowSettings }: { canvas: FlowCanvas; card: FlowRoutineCard; onShowSettings: (card: FlowRoutineCard, step: FlowStep) => void }) {
  const tr = useTr()
  if (card.steps.length === 0) return null
  const name = (id: string) => {
    const custom = stepNameOf(canvas, id)
    if (custom) return custom
    const media = canvas.mediaNodes.find((node) => mediaKey(node.id) === id)
    if (media) return tr(mediaTitle(media))
    return canvas.routines.find((routine) => routine.routine.botId === id)?.routine.name ?? id
  }
  return (
    <VStack gap={6}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Latest run')}</Text>
      {card.steps.map((step) => (
        <VStack key={step.id} gap={2} className="flow-step">
          <HStack gap={8}>
            <Text size="callout" truncate>{name(step.botId)}</Text>
            <Spacer />
            <StatusChip status={step.status} />
          </HStack>
          {step.error ? <Text size="caption" tone="danger" lines={3}>{step.error}</Text> : null}
          <StepResult key={step.id} step={step} compact />
          <HStack gap={6}>
            <Spacer />
            <Button kind="secondary" onClick={() => onShowSettings(card, step)}>{tr('Settings')}</Button>
          </HStack>
        </VStack>
      ))}
    </VStack>
  )
}

function History({ card }: { card: FlowRoutineCard }) {
  const tr = useTr()
  if (card.recentRuns.length === 0) return null
  return (
    <VStack gap={4}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Run history')}</Text>
      {card.recentRuns.map((run) => (
        <HStack key={run.id} gap={8}>
          <Text size="caption">{tr(runLabel(run))}</Text>
          <Spacer />
          <Text size="caption" tone="secondary">{new Date(run.createdAt).toLocaleString()}</Text>
        </HStack>
      ))}
    </VStack>
  )
}

function BotPanel({ botId, bot, canvas, busy, offline, readOnly, targets, onConnectFrom, onRemoveNode }: Props & { botId: string; bot?: Bot }) {
  const tr = useTr()
  const step = latestStep(canvas, botId)
  const options = targets(botId)
  const removable = canvas.placedBotIds.includes(botId) && !canvas.links.some((l) => l.fromBotId === botId || l.toBotId === botId)
  const title = stepNameOf(canvas, botKey(botId)) || bot?.name || tr('Unknown bot')
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{title}</Text>
      <Text size="callout" tone="secondary">{tr('Model')}: {bot?.model || bot?.backend || tr('Default')}</Text>
      <Text size="caption" tone="secondary">{tr('This step uses its agent’s model. Set a model on the link that reaches it to use a different one for a single step.')}</Text>
      <HStack gap={8}>
        <StatusChip status={step?.status ?? null} />
        <Text size="caption" tone="secondary">{tr(statusLabel(step?.status ?? null))}</Text>
      </HStack>
      <DropdownMenu
        title={tr('Hand its reply on to…')}
        className="flow-menu-button"
        items={() => options.map((target) => ({ title: target.name, action: () => onConnectFrom(botId, target.id) }))}
      >
        <Text>{options.length === 0 ? tr('No step to hand on to') : tr('Hand its reply on to…')}</Text>
      </DropdownMenu>
      {removable ? <Button kind="secondary" disabled={busy || offline || readOnly} onClick={() => onRemoveNode(botKey(botId))}>{tr('Remove from the canvas')}</Button> : null}
      {step ? <StepResult key={step.id} step={step} /> : null}
      {step?.error ? <Text size="caption" tone="danger" lines={4}>{step.error}</Text> : null}
    </VStack>
  )
}

/**
 * A picture or a clip step: its description, the model that makes the file and the choices that
 * model offers, plus what it may hand the file on to (host/src/media).
 * Ported from OpenBot 0.70.0 `WorkflowImageNode.tsx`'s inspector.
 */
function MediaPanel({ media, canvas, busy, offline, readOnly, fieldsFor, problemsFor, answerFor, checkFor, onSaveAnswer, onSaveCheck, targets, onConnectFrom, onSaveMedia, onRemoveMedia, onDuplicateMedia }: Props & { media: FlowMedia }) {
  const tr = useTr()
  const store = useStore()
  const nodeKey = mediaKey(media.id)
  const locked = busy || offline || readOnly
  const card = canvas.routines.find((c) => c.routine.id === media.routineId)
  const owner = card?.routine.botId ?? canvas.botId
  const step = card ? stepOf(card, nodeKey) : null
  const [answer, setAnswer] = useState<MediaAnswer | null>(null)
  const [failed, setFailed] = useState(false)
  const [draft, setDraft] = useState<MediaDraft>(() => mediaDraft(media))
  /** A length outside the model's short list opens the Custom slider (OpenBot 0.74.0). */
  const [customLength, setCustomLength] = useState(false)
  const kept = `${media.model}|${media.prompt}|${media.aspect}|${media.duration}|${media.resolution}|${media.transparent}|${media.voice}`
  // The host's copy changed (a save, another device): show it; an edit in the box is kept otherwise.
  useEffect(() => setDraft(mediaDraft(media)), [media.id, kept])
  useEffect(() => {
    let alive = true
    setAnswer(null)
    setFailed(false)
    void store.mediaModels(owner)
      .then((a) => alive && setAnswer(a))
      .catch(() => alive && setFailed(true))
    return () => {
      alive = false
    }
  }, [owner, store])
  const models = answer?.models.filter((model) => model.kinds.includes(media.kind)) ?? []
  const model = models.find((m) => m.id === draft.model) ?? null
  const current = mediaDraft(media)
  const changed = (Object.keys(draft) as (keyof MediaDraft)[]).some((key) => draft[key] !== current[key])
  const options = targets(nodeKey)
  const caps = model?.capabilities
  const durationList = caps?.durations ?? []
  const CUSTOM_LENGTH = -1
  const lengthMin = durationList.length ? Math.min(...durationList) : 0
  const lengthMax = durationList.length ? Math.max(...durationList) : 0
  // A saved length the model's short list doesn't offer stays on the Custom slider.
  useEffect(() => setCustomLength(durationList.length > 1 && !durationList.includes(draft.duration)),
    [media.id, draft.model, durationList.join(',')])
  const setLength = (value: number) => {
    if (value === CUSTOM_LENGTH) {
      setCustomLength(true)
      if (draft.duration < lengthMin || draft.duration > lengthMax) setDraft({ ...draft, duration: lengthMin })
    } else {
      setCustomLength(false)
      setDraft({ ...draft, duration: value })
    }
  }
  const pick = (chosen: MediaModel) => setDraft({ ...onModel(draft, chosen), prompt: draft.prompt })
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{media.name?.trim() || tr(media.kind === 'video' ? 'Clip step' : 'Picture step')}</Text>
      <HStack gap={8}>
        <StatusChip status={step?.status ?? null} />
        <Text size="caption" tone="secondary">{tr(statusLabel(step?.status ?? null))}</Text>
      </HStack>
      <Text size="caption" tone="secondary">{tr('The step makes a {kind} from this description; whatever the step before hands on is added to it.').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}</Text>
      <FieldTextArea
        value={draft.prompt}
        onChange={(prompt) => setDraft({ ...draft, prompt })}
        fields={fieldsFor(nodeKey)}
        problems={problemsFor(nodeKey)}
        disabled={locked}
        label="What to make"
        placeholder={tr('Describe the {kind} to make').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}
      />
      <StepFieldsToolbar
        nodeKey={nodeKey}
        disabled={locked}
        answerFor={answerFor}
        checkFor={checkFor}
        onSaveAnswer={onSaveAnswer}
        onSaveCheck={onSaveCheck}
      />
      <VStack gap={4}>
        <Text size="caption" tone="secondary">{tr('Model')}</Text>
        {failed ? (
          <Text size="caption" tone="attention">{tr("This computer's host is not answering.")}</Text>
        ) : !answer ? (
          <Text size="caption" tone="secondary">{tr('Checking what this bot can make…')}</Text>
        ) : !answer.agent ? (
          <Text size="caption" tone="secondary">{tr('This bot’s agent has no image tool. Pick one that does, or use ComfyUI.')}</Text>
        ) : models.length === 0 ? (
          <Text size="caption" tone="attention">{tr('This bot’s agent makes no {kind}.').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}</Text>
        ) : (
          <>
            {!answer.installed ? <Text size="caption" tone="attention">{tr('This bot’s agent is not installed on this computer yet.')}</Text> : null}
            {models.length > 1 ? (
              <DropdownMenu
                className="flow-menu-button"
                items={() => models.map((m) => ({ title: m.name, action: () => pick(m) }))}
              >
                <Text>{model?.name ?? tr('Pick a model')}</Text>
              </DropdownMenu>
            ) : (
              <Text size="callout" tone="secondary">{model?.name ?? draft.model}</Text>
            )}
          </>
        )}
      </VStack>
      {model && caps ? (
        <VStack gap={8}>
          <VStack gap={4}>
            <Text size="caption" tone="secondary">{tr('Aspect ratio')}</Text>
            <ChipChoice label="Aspect ratio" selection={draft.aspect} options={caps.aspects.map((a) => ({ id: a, label: a }))} onChange={(aspect) => setDraft({ ...draft, aspect })} />
          </VStack>
          {media.kind === 'video' && durationList.length > 0 ? (
            <VStack gap={4}>
              <Text size="caption" tone="secondary">{tr('Length')}</Text>
              <ChipChoice
                label="Length"
                selection={customLength ? CUSTOM_LENGTH : draft.duration}
                options={[
                  ...durationList.map((d) => ({ id: d, label: `${d}s` })),
                  ...(durationList.length > 1 ? [{ id: CUSTOM_LENGTH, label: tr('Custom') }] : []),
                ]}
                onChange={setLength}
              />
              {customLength && durationList.length > 1 ? (
                <HStack gap={8} align="center" className="flow-length-custom">
                  <input
                    type="range"
                    min={lengthMin}
                    max={lengthMax}
                    step={1}
                    value={Math.min(lengthMax, Math.max(lengthMin, draft.duration))}
                    aria-label={tr('Custom length')}
                    disabled={locked}
                    onChange={(event) => setDraft({ ...draft, duration: Number(event.target.value) })}
                  />
                  <Text size="caption" tone="secondary">{draft.duration}s</Text>
                </HStack>
              ) : null}
            </VStack>
          ) : null}
          {media.kind === 'video' && caps.resolutions.length > 0 ? (
            <VStack gap={4}>
              <Text size="caption" tone="secondary">{tr('Resolution')}</Text>
              <ChipChoice label="Resolution" selection={draft.resolution} options={caps.resolutions.map((r) => ({ id: r, label: r }))} onChange={(resolution) => setDraft({ ...draft, resolution })} />
            </VStack>
          ) : null}
          {media.kind === 'video' && caps.voices ? (
            <VStack gap={4}>
              <Text size="caption" tone="secondary">{tr('Voice')}</Text>
              <ChipChoice
                label="Voice"
                selection={draft.voice ?? ''}
                options={[{ id: '', label: tr('None') }, ...caps.voices.map((v) => ({ id: v.id, label: v.name }))]}
                onChange={(voice) => setDraft({ ...draft, voice: voice || null })}
              />
            </VStack>
          ) : null}
          {media.kind === 'image' && caps.transparent ? (
            <Switch on={draft.transparent} onChange={(transparent) => setDraft({ ...draft, transparent })}>{tr('Transparent background')}</Switch>
          ) : null}
        </VStack>
      ) : null}
      <HStack gap={8}>
        <Button disabled={locked || !changed} onClick={() => onSaveMedia(media, draft)}>{tr('Save')}</Button>
        <Spacer />
        <Button kind="secondary" disabled={locked} onClick={() => onDuplicateMedia(media)}>{tr('Duplicate')}</Button>
        <Button kind="secondary" disabled={locked} onClick={() => onRemoveMedia(media)}>{tr('Remove the step')}</Button>
      </HStack>
      <DropdownMenu
        title={tr('Hand its file on to…')}
        className="flow-menu-button"
        items={() => options.map((target) => ({ title: target.name, action: () => onConnectFrom(nodeKey, target.id) }))}
      >
        <Text>{options.length === 0 ? tr('No step to hand on to') : tr('Hand its file on to…')}</Text>
      </DropdownMenu>
      {step ? <StepResult key={step.id} step={step} /> : null}
      {step?.error ? <Text size="caption" tone="danger" lines={4}>{step.error}</Text> : null}
    </VStack>
  )
}
