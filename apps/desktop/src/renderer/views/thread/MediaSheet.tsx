import { useEffect, useState } from 'react'
import { firstChoice, modelDetail, modelsOf, onModel, mediaDraft, mediaBot, type MediaAnswer, type MediaChoice, type MediaKind, type MediaModel } from '@shared/media'
import type { Bot } from '@shared/models'
import { Button, ChipChoice, ChoiceList, SegmentedChoice, Spinner, Switch } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'

/**
 * The picker behind `/image` and `/video`: the models this bot's agent can make files with, the
 * frame, and for a video its length, resolution and voice. It writes the command into the box, so
 * the person only describes what to make (host/src/media).
 * Ported from OpenBot 0.70.0 `WorkflowMediaModelPicker.tsx` and `WorkflowImageNode.tsx`.
 */
export function MediaSheet({ botId, kind: asked, draft, open, onClose, onUse }: {
  botId: string
  /** The command picked in the menu. */
  kind: MediaKind
  /** The box's text, whose description is kept. */
  draft: string
  open: boolean
  onClose: () => void
  onUse: (message: string) => void
}) {
  const store = useStore()
  const tr = useTr()
  const bot = mediaBot(store.bots.get(botId) ?? null)
  const [answer, setAnswer] = useState<MediaAnswer | null>(null)
  const [kind, setKind] = useState<MediaKind>(asked)
  const [choice, setChoice] = useState<MediaChoice | null>(null)
  const [failed, setFailed] = useState(false)

  // Every opening asks again: a different agent account may answer, and a bot's agent can change.
  useEffect(() => {
    if (!open || !bot) return
    let alive = true
    setAnswer(null)
    setFailed(false)
    setKind(asked)
    setChoice(null)
    void store.mediaModels(botId)
      .then((a) => {
        if (!alive) return
        setAnswer(a)
        setChoice(firstChoice(a, asked))
      })
      .catch(() => alive && setFailed(true))
    return () => {
      alive = false
    }
  }, [open, asked, botId, bot, store])

  const models = modelsOf(answer, kind)
  const model = models.find((m) => m.id === choice?.model) ?? null
  const use = () => {
    if (!choice) return
    onUse(mediaDraft(choice, draft))
    onClose()
  }

  const pickKind = (next: MediaKind) => {
    setKind(next)
    setChoice(firstChoice(answer, next))
  }

  return (
    <Sheet open={open} onClose={onClose} width={420} maxHeight="80vh">
      <SheetPage>
        <ModalHeader
          title={kind === 'image' ? tr('Image') : tr('Video')}
          subtitle={answer?.agent ? tr('Made by {agent}, paid with its subscription').replace('{agent}', answer.agent) : undefined}
          icon={kind === 'image' ? 'sparkles' : 'film'}
          trailing={<Button disabled={!choice} onClick={use}>{tr('Use')}</Button>}
        />
        <VStack gap={14} className="media-sheet">
          <SegmentedChoice
            selection={kind}
            options={[{ id: 'image' as const, label: 'Image' }, { id: 'video' as const, label: 'Video' }]}
            onChange={pickKind}
          />
          {failed ? (
            <Text tone="attention">{tr("This computer's host is not answering.")}</Text>
          ) : !answer ? (
            <HStack gap={8}><Spinner size={16} /><Text tone="secondary">{tr('Checking what this bot can make…')}</Text></HStack>
          ) : !answer.agent ? (
            <Text tone="secondary">{tr('This bot’s agent has no image tool. Pick one that does, or use ComfyUI.')}</Text>
          ) : models.length === 0 ? (
            <Text tone="secondary">{tr('This bot’s agent makes no {kind}.').replace('{kind}', tr(kind === 'image' ? 'Image' : 'Video').toLowerCase())}</Text>
          ) : (
            <>
              {!answer.installed ? (
                <Text size="callout" tone="attention">{tr("This bot’s agent is not installed on this computer yet.")}</Text>
              ) : null}
              {models.length > 1 ? (
                <ChoiceList
                  selection={choice?.model ?? ''}
                  options={models.map((m) => ({ id: m.id, label: m.name, detail: modelDetail(m, answer, tr) }))}
                  onChange={(id) => {
                    const next = models.find((m) => m.id === id)
                    if (next && choice) setChoice(onModel(choice, next))
                  }}
                />
              ) : model ? (
                <Text size="callout" tone="secondary">{modelDetail(model, answer, tr)}</Text>
              ) : null}
              {choice && model ? <MediaSettings choice={choice} model={model} onChange={setChoice} /> : null}
            </>
          )}
          {choice ? (
            <Text size="caption" tone="secondary" className="media-preview">{mediaDraft(choice, draft)}</Text>
          ) : null}
        </VStack>
      </SheetPage>
    </Sheet>
  )
}

/** A frame, and for a video its length, resolution and voice; only what the model makes. */
function MediaSettings({ choice, model, onChange }: { choice: MediaChoice; model: MediaModel; onChange: (choice: MediaChoice) => void }) {
  const tr = useTr()
  const caps = model.capabilities
  return (
    <VStack gap={12}>
      <VStack gap={6}>
        <Text size="callout" tone="secondary">{tr('Aspect ratio')}</Text>
        <ChipChoice label="Aspect ratio" selection={choice.aspect} options={caps.aspects.map((a) => ({ id: a, label: a }))} onChange={(aspect) => onChange({ ...choice, aspect })} />
      </VStack>
      {choice.kind === 'video' && caps.durations.length > 0 ? (
        <VStack gap={6}>
          <Text size="callout" tone="secondary">{tr('Length')}</Text>
          <ChipChoice
            label="Length"
            selection={choice.duration}
            options={caps.durations.map((d) => ({ id: d, label: `${d}s` }))}
            onChange={(duration) => onChange({ ...choice, duration })}
          />
        </VStack>
      ) : null}
      {choice.kind === 'video' && caps.resolutions.length > 0 ? (
        <VStack gap={6}>
          <Text size="callout" tone="secondary">{tr('Resolution')}</Text>
          <ChipChoice label="Resolution" selection={choice.resolution} options={caps.resolutions.map((r) => ({ id: r, label: r }))} onChange={(resolution) => onChange({ ...choice, resolution })} />
        </VStack>
      ) : null}
      {choice.kind === 'video' && caps.voices ? (
        <VStack gap={6}>
          <Text size="callout" tone="secondary">{tr('Voice')}</Text>
          <ChipChoice
            label="Voice"
            selection={choice.voice ?? ''}
            options={[{ id: '', label: tr('None') }, ...caps.voices.map((v) => ({ id: v.id, label: v.name }))]}
            onChange={(voice) => onChange({ ...choice, voice: voice || null })}
          />
        </VStack>
      ) : null}
      {choice.kind === 'image' && caps.transparent ? (
        <Switch on={choice.transparent} onChange={(transparent) => onChange({ ...choice, transparent })}>{tr('Transparent background')}</Switch>
      ) : null}
    </VStack>
  )
}

/** Whether the picker is offered at all: a group has no agent of its own. */
export function mediaOffered(bot: Bot | null): boolean {
  return mediaBot(bot) !== null
}
