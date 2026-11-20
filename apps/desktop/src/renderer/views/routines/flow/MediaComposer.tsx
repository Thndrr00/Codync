import { useEffect, useState } from 'react'
import { onModel, type MediaAnswer, type MediaKind } from '@shared/media'
import { Button, ChipChoice, DropdownMenu, IconButton, Switch } from '../../../components/Controls'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { mediaKey, mediaOf, mediaTitle, type FlowMedia } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { useStore } from '../../../store/context'
import type { CanvasWithGroups } from './flow-groups'
import type { FlowTarget, MediaSettings } from './FlowInspector'
import { settingsOf } from './flow-settings'

/** The composer shown under a selected picture or clip card: description, pictures, model and Run
 *  together (OpenBot 0.75.0). It reuses the inspector's shared controls and the saved step snapshot;
 *  the host still owns the settings and the run. */

interface Props {
  media: FlowMedia
  canvas: CanvasWithGroups
  busy: boolean
  offline: boolean
  targets: (from: string) => FlowTarget[]
  onSaveMedia: (media: FlowMedia, settings: MediaSettings) => void
  onRunFromStep: (nodeKey: string) => void
  onClose: () => void
}

type MediaDraft = MediaSettings & { kind: MediaKind }

const draftOf = (media: FlowMedia): MediaDraft => ({
  kind: media.kind,
  model: media.model,
  prompt: media.prompt,
  aspect: media.aspect,
  duration: media.duration,
  resolution: media.resolution,
  transparent: media.transparent,
  voice: media.voice,
})

export function MediaComposer({ media, canvas, busy, offline, targets, onSaveMedia, onRunFromStep, onClose }: Props) {
  const tr = useTr()
  const store = useStore()
  const [answer, setAnswer] = useState<MediaAnswer | null>(null)
  const [failed, setFailed] = useState(false)
  const [draft, setDraft] = useState<MediaDraft>(() => draftOf(media))
  useEffect(() => setDraft(draftOf(media)), [media.id, media.model, media.prompt, media.aspect, media.duration, media.resolution, media.transparent, media.voice])
  const card = canvas.routines.find((routine) => routine.routine.id === media.routineId)
  const owner = card?.routine.botId ?? canvas.botId
  useEffect(() => {
    let alive = true
    setAnswer(null)
    setFailed(false)
    void store.mediaModels(owner)
      .then((models) => alive && setAnswer(models))
      .catch(() => alive && setFailed(true))
    return () => { alive = false }
  }, [owner, store])

  const key = mediaKey(media.id)
  const models = answer?.models.filter((model) => model.kinds.includes(media.kind)) ?? []
  const model = models.find((choice) => choice.id === draft.model) ?? null
  const caps = model?.capabilities
  const original = draftOf(media)
  const changed = (Object.keys(draft) as (keyof MediaSettings)[]).some((name) => draft[name] !== original[name])
  // The newest result's saved settings, reused from the previous pass; an older run has none.
  const step = canvas.routines.flatMap((routine) => routine.steps).find((candidate) => candidate.botId === key)
  const saved = step ? settingsOf(canvas.stepSettings, step.id) : null
  // Pictures whose file is handed to this step by an incoming media link.
  const pictures = canvas.links
    .filter((link) => link.toBotId === key && link.fromBotId.startsWith('media:'))
    .map((link) => mediaOf(canvas, link.fromBotId))
    .filter((item): item is FlowMedia => item !== null)
  const connected = targets(key)

  return (
    <VStack gap={8} className="flow-composer">
      <HStack gap={8} align="center">
        <Text weight="semibold">{media.name?.trim() || tr(media.kind === 'video' ? 'Clip step' : 'Picture step')}</Text>
        <Spacer />
        <IconButton title={tr('Close')} icon="xmark" onClick={onClose} />
      </HStack>
      <textarea
        className="routine-input flow-composer-prompt"
        aria-label={tr('What to make')}
        placeholder={tr('Describe the {kind} to make').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}
        value={draft.prompt}
        maxLength={8000}
        disabled={offline}
        onChange={(event) => setDraft({ ...draft, prompt: event.target.value })}
      />
      {pictures.length > 0 ? (
        <VStack gap={4}>
          <Text size="caption" tone="secondary">{tr('Connected pictures')}</Text>
          <HStack gap={6} wrap>
            {pictures.map((picture) => (
              <span key={picture.id} className="flow-composer-picture" title={picture.prompt}>
                <Text size="caption" truncate>{picture.name?.trim() || tr(mediaTitle(picture))}</Text>
              </span>
            ))}
          </HStack>
        </VStack>
      ) : null}
      <VStack gap={4}>
        <Text size="caption" tone="secondary">{tr('Model')}</Text>
        {failed ? (
          <Text size="caption" tone="attention">{tr("This computer's host is not answering.")}</Text>
        ) : !answer ? (
          <Text size="caption" tone="secondary">{tr('Checking what this bot can make…')}</Text>
        ) : models.length === 0 ? (
          <Text size="caption" tone="attention">{tr('This bot’s agent makes no {kind}.').replace('{kind}', tr(media.kind === 'video' ? 'clip' : 'picture'))}</Text>
        ) : (
          <DropdownMenu
            className="flow-menu-button"
            items={() => models.map((choice) => ({ title: choice.name, action: () => setDraft({ ...onModel(draft, choice), prompt: draft.prompt }) }))}
          >
            <Text>{model?.name ?? tr('Pick a model')}</Text>
          </DropdownMenu>
        )}
      </VStack>
      {model && caps ? (
        <VStack gap={6}>
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
              <ChipChoice label="Voice" selection={draft.voice ?? ''} options={[{ id: '', label: tr('None') }, ...caps.voices.map((v) => ({ id: v.id, label: v.name }))]} onChange={(voice) => setDraft({ ...draft, voice: voice || null })} />
            </VStack>
          ) : null}
          {media.kind === 'image' && caps.transparent ? (
            <Switch on={draft.transparent} onChange={(transparent) => setDraft({ ...draft, transparent })}>{tr('Transparent background')}</Switch>
          ) : null}
        </VStack>
      ) : null}
      {saved ? (
        <Text size="caption" tone="secondary" truncate>
          {tr('Last run: {model} · {aspect}').replace('{model}', saved.model ?? draft.model).replace('{aspect}', saved.aspect ?? draft.aspect)}
        </Text>
      ) : null}
      {connected.length > 0 ? (
        <Text size="caption" tone="secondary" truncate>
          {tr('Hands its file on to {count} step(s)').replace('{count}', String(connected.length))}
        </Text>
      ) : null}
      <HStack gap={8}>
        <Button disabled={busy || offline || !changed} onClick={() => onSaveMedia(media, draft)}>{tr('Save')}</Button>
        <Spacer />
        <Button kind="secondary" disabled={busy || offline} onClick={() => onRunFromStep(key)}>{tr('Run')}</Button>
      </HStack>
    </VStack>
  )
}
