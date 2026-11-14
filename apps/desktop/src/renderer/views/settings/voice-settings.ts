import { voiceEngine, voiceEngineIds } from '../../../shared/voice-engine'
import type { HostClient } from '../../client/host-client'
import { pref, type Pref } from '../../lib/prefs'

// Voice on the user's own provider keys (host/src/voice.rs, kit Voice.swift + VoiceSettings).
// The keys stay on the computer; these are this device's choices.

export type VoiceProvider = 'openai' | 'gemini' | 'elevenlabs' | 'xai' | 'groq'

/** Who reads a reply aloud: the call engine's own provider, xAI's speech route, or NeuTTS on the computer. */
export type SpeechOutput = 'provider' | 'xai' | 'neutts'

/** The provider's newest model for each voice mode, as the computer last found them. */
export interface VoiceDefaults {
  realtime: string
  transcribe: string
  speech: string
}

export interface VoiceStatus {
  providers: { provider: VoiceProvider; configured: boolean; defaults?: VoiceDefaults | null }[]
}

export interface VoiceModels {
  /** Realtime conversation models. */
  models: string[]
  transcribe: string[]
  speech: string[]
  defaults?: VoiceDefaults | null
}

/** The voices the computer's key can read with (ElevenLabs). */
export interface VoiceVoices {
  voices: { id: string; name: string }[]
}

/** The local voice server this computer's speech goes through (host `voiceServer`). */
export interface VoiceServer {
  /** The saved URL; empty when none is saved here. */
  baseUrl: string
  transcribeModel: string
  speechModel: string
  keySaved: boolean
  /** Where speech requests actually go: the saved server, `CODYNC_OPENAI_BASE_URL`, or OpenAI. */
  effective: string
  /** `effective` is not api.openai.com: Realtime is unavailable meanwhile. */
  custom: boolean
  /** A developer's `CODYNC_OPENAI_BASE_URL` on this computer, when set. */
  environment?: string | null
  suggestions: string[]
}

/** The models common local servers serve, until the computer reports its own list. */
export const localModelSuggestions = ['parakeet-tdt-0.6b-v3', 'parakeet-unified-en-0.6b', 'whisper-turbo', 'cohere-transcribe-03-2026']

export const voiceInfo: Record<VoiceProvider, {
  name: string
  keyPage: string
  keyPrompt: string
  /** Used until the user picks one, and until the computer has said what's newest. */
  defaults: VoiceDefaults
  voices: string[]
}> = {
  openai: {
    name: 'OpenAI',
    keyPage: 'https://platform.openai.com/api-keys',
    keyPrompt: 'sk-…',
    defaults: { realtime: 'gpt-realtime-2.1', transcribe: 'gpt-transcribe', speech: 'gpt-4o-mini-tts' },
    voices: ['marin', 'cedar', 'alloy', 'ash', 'ballad', 'coral', 'echo', 'sage', 'shimmer', 'verse'],
  },
  gemini: {
    name: 'Gemini',
    keyPage: 'https://aistudio.google.com/apikey',
    keyPrompt: 'AIza…',
    defaults: { realtime: 'gemini-3.8-live', transcribe: 'gemini-3.8-flash', speech: 'gemini-3.8-flash-tts' },
    voices: ['Kore', 'Puck', 'Charon', 'Fenrir', 'Aoede', 'Leda', 'Orus', 'Zephyr'],
  },
  elevenlabs: {
    name: 'ElevenLabs',
    keyPage: 'https://elevenlabs.io/app/settings/api-keys',
    keyPrompt: 'sk_…',
    // Speech only: there is no live conversation, and the account's voices come from the computer.
    defaults: { realtime: '', transcribe: 'scribe_v1', speech: 'eleven_flash_v2_5' },
    voices: ['21m00Tcm4TlvDq8ikWAM'],
  },
  // Speech only (OpenBot 0.60): reads replies aloud; it hears nothing.
  xai: {
    name: 'xAI',
    keyPage: 'https://console.x.ai',
    keyPrompt: 'xai-…',
    defaults: { realtime: '', transcribe: '', speech: 'xai-tts' },
    voices: ['eve', 'ara', 'leo', 'rex', 'sal'],
  },
  // Dictation only (OpenBot 0.60): hears speech; it reads nothing.
  groq: {
    name: 'Groq',
    keyPage: 'https://console.groq.com/keys',
    keyPrompt: 'gsk_…',
    defaults: { realtime: '', transcribe: 'whisper-large-v3-turbo', speech: '' },
    voices: [],
  },
}

/** Providers that hold a live conversation; the others carry Speech, hear or read only. */
export const supportsRealtime = (p: VoiceProvider) => p === 'openai' || p === 'gemini'

/** Groq hears speech; xAI reads replies aloud and hears nothing. */
export const canDictate = (p: VoiceProvider) => p !== 'xai'
export const canSpeak = (p: VoiceProvider) => p !== 'groq'

/** What can carry a whole call: it must hear and read aloud. xAI and Groq each do half. */
export const isCallProvider = (p: VoiceProvider) => canDictate(p) && canSpeak(p)

export const isVoiceProvider = (s: string): s is VoiceProvider =>
  s === 'openai' || s === 'gemini' || s === 'elevenlabs' || s === 'xai' || s === 'groq'
export const configured = (s: VoiceStatus, p: VoiceProvider) => s.providers.some((x) => x.provider === p && x.configured)

/** `device` (on-device speech) or a provider. */
function enginePref(stored: Pref<string>): Pref<string> {
  return { ...stored, get: () => voiceEngine(stored.get(), window.codync.platform) }
}

/** The providers a call can use, then the ones that only dictate (Groq). */
export const engineOptions = () => voiceEngineIds(window.codync.platform)
  .filter((id) => !isVoiceProvider(id) || isCallProvider(id))
  .map((id) => ({ id, label: isVoiceProvider(id) ? voiceInfo[id].name : 'On device' }))

export const dictationEngineOptions = () => voiceEngineIds(window.codync.platform)
  .filter((id) => !isVoiceProvider(id) || canDictate(id))
  .map((id) => ({ id, label: isVoiceProvider(id) ? voiceInfo[id].name : 'On device' }))

const storedCallEngine = pref('callEngine', 'device')
/** Keep an old/default device choice available to call startup's configured-provider fallback. */
export const callEngineChoice = storedCallEngine.get
export const engine = enginePref(storedCallEngine)
/** The same choice for dictation into the message box, picked beside its mic. */
export const dictationEngine = enginePref(pref('dictationEngine', 'device'))
/** On-device reading speed (AVSpeechUtterance's scale: 0.5 is normal). */
export const DEFAULT_RATE = 0.5
export const rate = pref('callRate', DEFAULT_RATE)
/** Seconds of silence that end what you say. */
export const pause = pref('callPause', 1.5)
/** On-device recognition language (a locale identifier); '' follows the system. */
export const language = pref('callLanguage', '')

/** Picker options: the system's language, then every language the recognizer supports, by name. */
export function languageOptions(supported: string[]) {
  const names = new Intl.DisplayNames([navigator.language], { type: 'language' })
  const name = (id: string) => {
    try {
      return names.of(id.replace('_', '-')) ?? id
    } catch {
      return id
    }
  }
  return [
    { id: '', label: 'System' },
    ...supported.map((id) => ({ id, label: name(id) })).sort((a, b) => a.label.localeCompare(b.label)),
  ]
}

const cache = new Map<string, Pref<unknown>>()
function keyed<T>(key: string, fallback: T): Pref<T> {
  if (!cache.has(key)) cache.set(key, pref<T>(key, fallback) as Pref<unknown>)
  return cache.get(key) as Pref<T>
}

/** `realtime` (a live conversation) or `speech` (speech to text, then the reply read aloud). */
export const mode = (p: VoiceProvider) => keyed<string>(`callMode.${p}`, 'realtime')

/** Speech: speech to text, then the reply read aloud. ElevenLabs has no live conversation. */
export const isSpeechMode = (p: VoiceProvider) => !supportsRealtime(p) || mode(p).get() === 'speech'

/** Speech output only: recognition/realtime keep the user's existing provider connection. */
export const speechOutput = keyed<SpeechOutput>('callSpeechOutput', 'provider')
/** Who reads replies aloud: the call engine's provider, xAI's own route, or NeuTTS. */
export const speechProvider = (p: VoiceProvider) => (speechOutput.get() === 'xai' ? 'xai' : p)
export const localSpeech = () => speechOutput.get() === 'neutts'
const voicePref = (p: VoiceProvider) => keyed<string | null>(`callVoice.${p}`, null)
const defaultsPref = (p: VoiceProvider) => keyed<VoiceDefaults | null>(`callDefaults.${p}`, null)
/** The provider's model list as last fetched with the user's key. */
export const modelsCache = (p: VoiceProvider) => keyed<VoiceModels | null>(`callModels.${p}`, null)
const modelPrefs = {
  realtime: (p: VoiceProvider) => keyed<string | null>(`callModel.${p}`, null),
  transcribe: (p: VoiceProvider) => keyed<string | null>(`callSpeechToText.${p}`, null),
  speech: (p: VoiceProvider) => keyed<string | null>(`callSpeechModel.${p}`, null),
}

/** The newest models as the computer last reported them, or the app's own until it has. */
export const voiceDefaults = (p: VoiceProvider) => defaultsPref(p).get() ?? voiceInfo[p].defaults

export function rememberDefaults(p: VoiceProvider, d: VoiceDefaults | null | undefined) {
  if (d) defaultsPref(p).set(d)
}

export type ModelKind = keyof VoiceDefaults

export const chosenModel = (p: VoiceProvider, kind: ModelKind) => modelPrefs[kind](p).get() ?? voiceDefaults(p)[kind]

/** Keeps a model the user picked; the default isn't stored, so a newer default reaches them. */
export function saveModel(p: VoiceProvider, kind: ModelKind, model: string) {
  modelPrefs[kind](p).set(!model || model === voiceDefaults(p)[kind] ? null : model)
}

export function chosenVoice(p: VoiceProvider) {
  const v = voicePref(p).get()
  if (v && (p === 'elevenlabs' || voiceInfo[p].voices.includes(v))) return v
  return voiceInfo[p].voices[0] ?? ''
}

export const saveVoice = (p: VoiceProvider, v: string) => voicePref(p).set(v)

// Minutes of realtime voice used this month, counted on this device: an estimate, not a bill.
const usageKey = (p: VoiceProvider, d: Date) => `callUsage.${p}.${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`

export function addVoiceUsage(seconds: number, p: VoiceProvider, at = new Date()) {
  if (seconds <= 0) return
  const u = keyed<number>(usageKey(p, at), 0)
  u.set(u.get() + seconds)
}

export const voiceMinutes = (p: VoiceProvider, at = new Date()) => Math.floor((keyed<number>(usageKey(p, at), 0).get() + 59) / 60)

// Host calls

/** Which providers the computer has keys for; the newest models it reports become the defaults. */
export async function voiceStatus(c: HostClient) {
  const status = await c.call<VoiceStatus>('voiceStatus')
  for (const entry of status.providers) rememberDefaults(entry.provider, entry.defaults)
  return status
}

/** Checks the key with the provider before the computer keeps it; an empty key removes it. */
export const setVoiceKey = (c: HostClient, p: VoiceProvider, key: string) => c.call<VoiceStatus>('setVoiceKey', { provider: p, key }, 30_000)

/** The local voice server's saved URL, models and key state. A phone may read it. */
export const voiceServer = (c: HostClient) => c.call<VoiceServer>('voiceServer', {}, 30_000)

/** Saves the local voice server (this computer only, like the voice keys). */
export const setVoiceServer = (c: HostClient, body: Partial<Pick<VoiceServer, 'baseUrl' | 'transcribeModel' | 'speechModel'> & { key: string }>) =>
  c.call<VoiceServer>('setVoiceServer', body, 30_000)

/** Tries the saved server and says why when it doesn't answer. */
export const testVoiceServer = (c: HostClient) =>
  c.call<{ ok: boolean; message: string; models?: string[]; effective?: string }>('testVoiceServer', {}, 30_000)

/** A short-lived credential for one call's audio session. */
export const voiceSession = async (c: HostClient, p: VoiceProvider, model: string, voice: string) =>
  (await c.call<{ credential: string }>('voiceSession', { provider: p, model, voice }, 30_000)).credential

/** The models the computer's key can use, per voice mode. */
export const voiceModels = (c: HostClient, p: VoiceProvider) => c.call<VoiceModels>('voiceModels', { provider: p }, 30_000)

/** The provider's own voices (ElevenLabs). */
export const voiceVoices = (c: HostClient, p: VoiceProvider) => c.call<VoiceVoices>('voiceVoices', { provider: p }, 30_000)

/** Text to speech for a stretch of a reply: base64 audio and its format (`aac`, `wav` or `mp3`). */
export async function voiceSpeak(c: HostClient, p: VoiceProvider, model: string, voice: string, text: string,
  options: { botId?: string; signal?: AbortSignal; local?: boolean } = {}) {
  const local = options.local ?? speechOutput.get() === 'neutts'
  const requestId = crypto.randomUUID()
  const cancel = () => { if (local) void c.call('voiceCancelSpeech', { requestId }, 10_000).catch(() => {}) }
  if (options.signal?.aborted) throw new DOMException('Speech cancelled', 'AbortError')
  options.signal?.addEventListener('abort', cancel, { once: true })
  try {
    return await c.call<{ audio: string; format: string }>('voiceSpeak', {
      provider: local ? 'neutts' : p, model, voice, text, requestId, ...(options.botId ? { botId: options.botId } : {}),
    }, 90_000)
  } finally { options.signal?.removeEventListener('abort', cancel) }
}

/** Pulls bounded WAV frames while the host owns the local synthesis worker. */
export async function* voiceSpeechStream(c: HostClient, text: string, botId: string, signal: AbortSignal) {
  const requestId = crypto.randomUUID()
  const cancel = () => { void c.call('voiceCancelSpeech', { requestId }, 10_000).catch(() => {}) }
  signal.addEventListener('abort', cancel, { once: true })
  try {
    if (signal.aborted) throw new DOMException('Speech cancelled', 'AbortError')
    await c.call('voiceStartSpeech', { requestId, text, botId }, 90_000)
    while (!signal.aborted) {
      const frame = await c.call<{ done: boolean; audio?: string; format?: string }>('voiceSpeechChunk', { requestId }, 90_000)
      if (signal.aborted) throw new DOMException('Speech cancelled', 'AbortError')
      if (frame.done) return
      if (!frame.audio || frame.format !== 'wav') throw new Error('Unreadable local speech frame')
      yield { audio: Uint8Array.from(atob(frame.audio), c => c.charCodeAt(0)), format: frame.format }
    }
  } finally { signal.removeEventListener('abort', cancel); cancel() }
}
