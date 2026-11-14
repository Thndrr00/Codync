import { useEffect, useRef, useState } from 'react'
import { LoopbackTransport, type HostClient } from '../../client/host-client'
import { Button, CardSection, IconButton, Reveal, Spinner, ValueRow } from '../../components/Controls'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { errorText, liveClient } from './parts'
import * as voice from './voice-settings'

/**
 * The local voice server: an OpenAI-compatible server the person runs on this computer (a
 * sherpa-onnx or whisper server, a Parakeet or Cohere Transcribe wrapper), which carries
 * dictation and spoken replies for the OpenAI engine. Codync runs no model itself. The setting
 * is the computer's, edited here (or over SSH from here), like the voice keys.
 */
export function LocalVoiceServerSettings() {
  const tr = useTr()
  const store = useStore()
  const [saved, setSaved] = useState<voice.VoiceServer | null>(null)
  const [url, setUrl] = useState('')
  const [transcribe, setTranscribe] = useState('')
  const [speech, setSpeech] = useState('')
  const [key, setKey] = useState('')
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null)
  const busyRef = useRef(false)
  const canEdit = store.client?.transport instanceof LoopbackTransport
  const notConnected = { ok: false, text: tr('Not connected to {name}.').replaceAll('{name}', () => store.hostName) }

  /** Fills the fields from the host's answer. An unsaved edit is never overwritten. */
  const show = (state: voice.VoiceServer) => {
    setSaved(state)
    setUrl(state.baseUrl)
    setTranscribe(state.transcribeModel)
    setSpeech(state.speechModel)
  }

  useEffect(() => {
    let cancelled = false
    const client = liveClient(store)
    if (!client) {
      setResult(notConnected)
      return
    }
    voice
      .voiceServer(client)
      .then((state) => {
        if (!cancelled) show(state)
      })
      .catch((e: unknown) => {
        if (!cancelled) setResult({ ok: false, text: errorText(e) })
      })
    return () => void (cancelled = true)
  }, [])

  const run = (work: (client: HostClient) => Promise<void>) => {
    if (busyRef.current) return
    const client = liveClient(store)
    if (!client) return setResult(notConnected)
    busyRef.current = true
    setBusy(true)
    setResult(null)
    void work(client)
      .catch((e: unknown) => setResult({ ok: false, text: errorText(e) }))
      .finally(() => {
        busyRef.current = false
        setBusy(false)
      })
  }

  /** Saves the form and tests what it saved, so a working address says so at once. */
  const save = () =>
    run(async (client) => {
      if (!(client.transport instanceof LoopbackTransport)) return
      const state = await voice.setVoiceServer(client, {
        baseUrl: url.trim(),
        transcribeModel: transcribe.trim(),
        speechModel: speech.trim(),
        ...(key.trim() ? { key: key.trim() } : {}),
      })
      show(state)
      setKey('')
      if (!state.baseUrl) return setResult({ ok: true, text: tr('Saved') })
      const tested = await voice.testVoiceServer(client)
      setResult({ ok: tested.ok, text: tested.ok ? tr('Server works') : `${tr("The server didn't answer")}: ${tested.message}` })
    })

  const test = () =>
    run(async (client) => {
      const tested = await voice.testVoiceServer(client)
      setResult({ ok: tested.ok, text: tested.ok ? tr('Server works') : `${tr("The server didn't answer")}: ${tested.message}` })
    })

  const removeKey = () =>
    run(async (client) => {
      if (!(client.transport instanceof LoopbackTransport)) return
      show(await voice.setVoiceServer(client, { key: '' }))
      setResult(null)
    })

  const suggestions = saved?.suggestions?.length ? saved.suggestions : voice.localModelSuggestions
  const field = (value: string, set: (v: string) => void, placeholder: string, label: string) => (
    <input
      className="settings-key-input"
      aria-label={tr(label)}
      placeholder={placeholder}
      value={value}
      spellCheck={false}
      autoComplete="off"
      disabled={!canEdit}
      onChange={(e) => set(e.target.value)}
    />
  )
  const chips = (set: (v: string) => void) => (
    <HStack gap={6} wrap className="voice-model-suggestions">
      {suggestions.map((model) => (
        <Button key={model} kind="secondary" onClick={() => set(model)} disabled={!canEdit}>
          {model}
        </Button>
      ))}
    </HStack>
  )
  const state = saved?.custom
    ? tr('In use: {url}').replaceAll('{url}', () => saved.effective)
    : saved?.baseUrl
      ? null
      : tr('No local voice server is set. Add the URL of the server on this computer.')

  return (
    <CardSection
      title={tr('Local voice server')}
      footer={tr('Runs on this computer: Codync sends your dictation and spoken replies there, and no audio goes to OpenAI.')}
    >
      <VStack gap={10}>
        {!canEdit ? (
          <Text size="footnote" tone="secondary">
            {tr('Set on the computer. Add or change it in Codync on {name}.').replaceAll('{name}', () => store.hostName)}
          </Text>
        ) : null}
        <ValueRow label={tr('Server URL')}>{field(url, setUrl, 'http://127.0.0.1:8890', 'Server URL')}</ValueRow>
        <VStack gap={6}>
          <ValueRow label={tr('Dictation model')} detail={tr('Type the model name your server serves.')}>
            {field(transcribe, setTranscribe, 'parakeet-tdt-0.6b-v3', 'Dictation model')}
          </ValueRow>
          {chips(setTranscribe)}
        </VStack>
        <VStack gap={6}>
          <ValueRow label={tr('Speech model')}>{field(speech, setSpeech, 'whisper-turbo', 'Speech model')}</ValueRow>
          {chips(setSpeech)}
        </VStack>
        <HStack gap={8} className="settings-row">
          <input
            type="password"
            className="settings-key-input"
            aria-label={tr('Optional key')}
            placeholder={saved?.keySaved ? tr('Saved. Type a new one to replace it.') : tr('Optional key')}
            value={key}
            spellCheck={false}
            autoComplete="off"
            disabled={!canEdit}
            onChange={(e) => setKey(e.target.value)}
          />
          {saved?.keySaved && canEdit ? <IconButton title="Remove key" icon="trash" onClick={removeKey} /> : null}
        </HStack>
        <div className="settings-row">
          <Text size="caption" tone="tertiary">{saved?.keySaved ? tr('Key saved') : tr('No key')}</Text>
          <Spacer />
          {busy ? <Spinner /> : null}
          <Button kind="secondary" onClick={test} disabled={busy || !saved?.custom}>
            {tr('Test server')}
          </Button>
          {canEdit ? (
            <Button onClick={save} disabled={busy || !liveClient(store)}>
              {tr('Save')}
            </Button>
          ) : null}
        </div>
        {state ? <Text size="footnote" tone="secondary">{state}</Text> : null}
        <Reveal show={result !== null}>
          {result ? (
            <HStack as="span" gap={5} className={`settings-key-result ${result.ok ? '' : 'failed'}`}>
              {result.text}
            </HStack>
          ) : null}
        </Reveal>
      </VStack>
    </CardSection>
  )
}
