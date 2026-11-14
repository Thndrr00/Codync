import { useEffect, useState } from 'react'
import { onModel, type MediaAnswer, type MediaKind, type MediaModel } from '@shared/media'
import type { Bot } from '@shared/models'
import { Button, ChipChoice, DropdownMenu, Switch } from '../../../components/Controls'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'
import {
  botKey, latestStep, mediaKey, statusLabel, stepOf,
  type FlowCanvas, type FlowLink, type FlowMedia, type FlowRoutineCard,
} from '../../../lib/routine-flow.ts'
import { useStore } from '../../../store/context'
import { useAgentModels } from '../../bots/agent-models'
import { AgentModelPicker } from '../../bots/AgentModelPicker'
import { runLabel } from '../routine'
import type { FlowSelection } from './FlowCanvas'
import { StatusChip } from './FlowNodeCard'

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
  onSaveInstruction: (link: FlowLink, instruction: string, model: string | null, modelOptions: Record<string, string>) => void
  onDisconnect: (link: FlowLink) => void
  onTest: (routineId: string) => void
  onSetEnabled: (routineId: string, enabled: boolean) => void
  onConnectFrom: (fromBotId: string, toBotId: string) => void
  onRemoveNode: (nodeKey: string) => void
  onAddMedia: (routineId: string, kind: MediaKind) => void
  onSaveMedia: (media: FlowMedia, settings: MediaSettings) => void
  onRemoveMedia: (media: FlowMedia) => void
  /** The steps and bots `from` may hand work to, in some routine of this canvas. */
  targets: (fromBotId: string) => FlowTarget[]
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

function LinkPanel({ link, bots, busy, offline, onSaveInstruction, onDisconnect }: Props & { link: FlowLink }) {
  const tr = useTr()
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
      <textarea
        className="routine-input flow-instruction"
        aria-label={tr('Instruction')}
        value={text}
        maxLength={8000}
        disabled={offline}
        onChange={(event) => setText(event.target.value)}
      />
      <VStack gap={4}>
        <Text size="caption" tone="secondary">{tr('Model')}</Text>
        <Text size="caption" tone="secondary">{tr("The model this step runs on. Default is the bot's own.")}</Text>
        <AgentModelPicker backend={target?.backend ?? ''} models={models} selection={model} onChange={setModelChoice}
          options={modelOptions} onOptionsChange={setModelOptions} sessionNote={false} />
      </VStack>
      <HStack gap={8}>
        <Button disabled={busy || offline || !changed} onClick={() => onSaveInstruction(link, text.trim(), model, modelOptions)}>{tr('Save')}</Button>
        <Spacer />
        <Button kind="secondary" disabled={busy || offline} onClick={() => onDisconnect(link)}>{tr('Disconnect')}</Button>
      </HStack>
    </VStack>
  )
}

function RoutinePanel({ card, canvas, busy, offline, onTest, onSetEnabled, onAddMedia }: Props & { card: FlowRoutineCard }) {
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
        <Switch on={routine.enabled} disabled={busy || offline} label={routine.name} onChange={(on) => onSetEnabled(routine.id, on)} />
      </HStack>
      {routine.nextRunAt ? <Text size="caption" tone="secondary">{tr('Next run')}: {new Date(routine.nextRunAt).toLocaleString()}</Text> : null}
      <Text size="caption" tone="secondary">{last ? `${tr('Last run')}: ${tr(runLabel(last))}` : tr('No runs yet')}</Text>
      {linked ? null : <Text size="caption" tone="secondary">{tr('Not connected')}</Text>}
      <Button disabled={busy || offline || !linked} onClick={() => onTest(routine.id)}>{tr('Test the flow')}</Button>
      <HStack gap={8}>
        <Button kind="secondary" disabled={busy || offline} onClick={() => onAddMedia(routine.id, 'image')}>{tr('Add a picture step')}</Button>
        <Button kind="secondary" disabled={busy || offline} onClick={() => onAddMedia(routine.id, 'video')}>{tr('Add a clip step')}</Button>
      </HStack>
      <Steps card={card} />
      <History card={card} />
    </VStack>
  )
}

/** The steps of the latest run, one line each, with a bot's error or the start of its answer. */
function Steps({ card }: { card: FlowRoutineCard }) {
  const tr = useTr()
  if (card.steps.length === 0) return null
  return (
    <VStack gap={6}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Latest run')}</Text>
      {card.steps.map((step) => (
        <VStack key={step.id} gap={2} className="flow-step">
          <HStack gap={8}>
            <Text size="callout" truncate>{step.botId}</Text>
            <Spacer />
            <StatusChip status={step.status} />
          </HStack>
          {step.error ? <Text size="caption" tone="danger" lines={3}>{step.error}</Text> : null}
          {step.output ? <Text size="caption" tone="secondary" lines={3}>{step.output}</Text> : null}
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

function BotPanel({ botId, bot, canvas, busy, offline, targets, onConnectFrom, onRemoveNode }: Props & { botId: string; bot?: Bot }) {
  const tr = useTr()
  const step = latestStep(canvas, botId)
  const options = targets(botId)
  const removable = canvas.placedBotIds.includes(botId) && !canvas.links.some((l) => l.fromBotId === botId || l.toBotId === botId)
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{bot?.name ?? tr('Unknown bot')}</Text>
      <Text size="callout" tone="secondary">{bot?.model || bot?.backend || ''}</Text>
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
      {removable ? <Button kind="secondary" disabled={busy || offline} onClick={() => onRemoveNode(botKey(botId))}>{tr('Remove from the canvas')}</Button> : null}
      {step?.error ? <Text size="caption" tone="danger" lines={4}>{step.error}</Text> : null}
    </VStack>
  )
}

/**
 * A picture or a clip step: its description, the model that makes the file and the choices that
 * model offers, plus what it may hand the file on to (host/src/media).
 * Ported from OpenBot 0.70.0 `WorkflowImageNode.tsx`'s inspector.
 */
function MediaPanel({ media, canvas, busy, offline, targets, onConnectFrom, onSaveMedia, onRemoveMedia }: Props & { media: FlowMedia }) {
  const tr = useTr()
  const store = useStore()
  const nodeKey = mediaKey(media.id)
  const card = canvas.routines.find((c) => c.routine.id === media.routineId)
  const owner = card?.routine.botId ?? canvas.botId
  const step = card ? stepOf(card, nodeKey) : null
  const [answer, setAnswer] = useState<MediaAnswer | null>(null)
  const [failed, setFailed] = useState(false)
  const [draft, setDraft] = useState<MediaDraft>(() => mediaDraft(media))
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
  const pick = (chosen: MediaModel) => setDraft({ ...onModel(draft, chosen), prompt: draft.prompt })
  return (
    <VStack gap={10} className="flow-inspector">
      <Text weight="semibold">{tr(media.kind === 'video' ? 'Clip step' : 'Picture step')}</Text>      <HStack gap={8}>
        <StatusChip status={step?.status ?? null} />
        <Text size="caption" tone="secondary">{tr(statusLabel(step?.status ?? null))}</Text>
      </HStack>
      <Text size="caption" tone="secondary">{tr('The step makes a {kind} from this description; whatever the step before hands on is added to it.').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}</Text>
      <textarea
        className="routine-input flow-instruction"
        aria-label={tr('What to make')}
        placeholder={tr('Describe the {kind} to make').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}
        value={draft.prompt}
        maxLength={8000}
        disabled={offline}
        onChange={(event) => setDraft({ ...draft, prompt: event.target.value })}
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
          {media.kind === 'video' && caps.durations.length > 0 ? (
            <VStack gap={4}>
              <Text size="caption" tone="secondary">{tr('Length')}</Text>
              <ChipChoice label="Length" selection={draft.duration} options={caps.durations.map((d) => ({ id: d, label: `${d}s` }))} onChange={(duration) => setDraft({ ...draft, duration })} />
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
        <Button disabled={busy || offline || !changed} onClick={() => onSaveMedia(media, draft)}>{tr('Save')}</Button>
        <Spacer />
        <Button kind="secondary" disabled={busy || offline} onClick={() => onRemoveMedia(media)}>{tr('Remove the step')}</Button>
      </HStack>
      <DropdownMenu
        title={tr('Hand its file on to…')}
        className="flow-menu-button"
        items={() => options.map((target) => ({ title: target.name, action: () => onConnectFrom(nodeKey, target.id) }))}
      >
        <Text>{options.length === 0 ? tr('No step to hand on to') : tr('Hand its file on to…')}</Text>
      </DropdownMenu>
      {step?.error ? <Text size="caption" tone="danger" lines={4}>{step.error}</Text> : null}
      {step?.output ? <Text size="caption" tone="secondary" lines={4}>{step.output}</Text> : null}
    </VStack>
  )
}
