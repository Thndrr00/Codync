// Making a picture or a clip with the bot's own agent (host/src/media). The host's `mediaModels`
// says which models, frames and lengths that agent can work with; the picker turns a choice into
// the `/image …` or `/video …` message that carries it, and the host writes the request from it.
// Ported from OpenBot 0.70.0 `packages/ui/src/features/workflows/WorkflowMediaModelPicker.tsx`.
import type { Bot } from './models'

export type MediaKind = 'image' | 'video'

/** One model a bot's agent makes files with. */
export interface MediaModel {
  id: string
  name: string
  /** The company that makes the model. */
  maker: string
  /** The subscription that pays for the file. */
  subscription: string
  /** What the model makes. */
  kinds: MediaKind[]
  aspects: string[]
  /** An image of this model can have a clear background. */
  transparent: boolean
  /** A video of this model can speak in a preset voice. */
  voices: boolean
  capabilities: {
    aspects: string[]
    durations: number[]
    resolutions: string[]
    transparent: boolean
    /** Null for a model whose videos don't speak. */
    voices: { id: string; name: string }[] | null
  }
}

/** The host's answer to `mediaModels`: null `agent` when the bot's agent has no image tool. */
export interface MediaAnswer {
  agent: string | null
  installed: boolean
  models: MediaModel[]
}

/** What the picker chose; the flags of the message follow it. */
export interface MediaChoice {
  kind: MediaKind
  model: string
  aspect: string
  /** Seconds; only a video has one. */
  duration: number
  /** `480p`, `720p`…; only a video has one. */
  resolution: string
  transparent: boolean
  voice: string | null
}

/** The flags and their values, in the order the host reads them. */
const VALUED = ['--model', '--aspect', '--duration', '--resolution', '--voice', '--first', '--last', '--ref']

/** The models of a kind, the first one chosen by default. */
export function modelsOf(answer: MediaAnswer | null, kind: MediaKind): MediaModel[] {
  return answer?.models.filter((m) => m.kinds.includes(kind)) ?? []
}

/** The choice the picker starts on: the kind's first model at its first frame, length and resolution. */
export function firstChoice(answer: MediaAnswer | null, kind: MediaKind): MediaChoice | null {
  const model = modelsOf(answer, kind)[0]
  if (!model) return null
  return {
    kind,
    model: model.id,
    aspect: model.capabilities.aspects[0] ?? '1:1',
    duration: model.capabilities.durations[0] ?? 0,
    resolution: model.capabilities.resolutions[0] ?? '',
    transparent: false,
    voice: null,
  }
}

/** The same choice on another model, keeping only the settings the new one makes. */
export function onModel(choice: MediaChoice, model: MediaModel): MediaChoice {
  const aspect = model.capabilities.aspects.includes(choice.aspect) ? choice.aspect : (model.capabilities.aspects[0] ?? '1:1')
  return {
    ...choice,
    model: model.id,
    aspect,
    duration: model.capabilities.durations.includes(choice.duration) ? choice.duration : (model.capabilities.durations[0] ?? 0),
    resolution: model.capabilities.resolutions.includes(choice.resolution) ? choice.resolution : (model.capabilities.resolutions[0] ?? ''),
    voice: choice.voice && model.capabilities.voices?.some((v) => v.id === choice.voice) ? choice.voice : null,
    transparent: choice.transparent && model.capabilities.transparent,
  }
}

/** The words of `draft` that are the description: after the command and its flags. */
export function mediaPrompt(draft: string): string {
  const words = draft.trim().split(/\s+/).filter(Boolean)
  // A command being typed (`/`, `/ima`, `/image`) is not part of the description.
  if (words[0]?.startsWith('/')) words.shift()
  let i = 0
  while (i < words.length) {
    const word = words[i]!
    if (word === '--transparent') i += 1
    else if (VALUED.includes(word)) i += 2
    else break
  }
  return words.slice(i).join(' ')
}

/** The chat message that asks for `choice`, keeping the description already typed in `draft`. */
export function mediaDraft(choice: MediaChoice, draft: string): string {
  const flags = [`--model ${choice.model}`, `--aspect ${choice.aspect}`]
  if (choice.kind === 'video') {
    flags.push(`--duration ${choice.duration}`, `--resolution ${choice.resolution}`)
    if (choice.voice) flags.push(`--voice ${choice.voice}`)
  } else if (choice.transparent) {
    flags.push('--transparent')
  }
  const prompt = mediaPrompt(draft)
  return `/${choice.kind} ${flags.join(' ')}${prompt ? ` ${prompt}` : ' '}`
}

/**
 * The detail under a model: who makes it and what pays for it, then how a bot with that agent is
 * set up when the agent's program is missing.
 */
export function modelDetail(model: MediaModel, answer: MediaAnswer | null, tr: (text: string) => string): string {
  const base = `${model.maker} · ${model.subscription}`
  return answer && !answer.installed ? `${base} · ${tr('not installed')}` : base
}

/** Whether a bot can use the picker at all: only its own agent's models exist, and a group has none. */
export function mediaBot(bot: Bot | null): Bot | null {
  return bot && bot.kind !== 'group' ? bot : null
}
