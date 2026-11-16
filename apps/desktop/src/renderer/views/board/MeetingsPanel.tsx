import { useEffect, useRef, useState } from 'react'
import { Button, IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import type { BotStore } from '../../store/bot-store'
import { errorText } from '../bots/drafts'
import { Field } from '../marketplace/pieces'
import { micPCM16k, toBase64 } from '../call/voice'

interface MeetingSummary {
  id: string
  title: string
  startedAt: number
  finishedAt: number
  chars: number
  hasMinutes: boolean
}

interface MeetingNote extends MeetingSummary {
  transcript: string
  minutes?: string
}

interface Recording {
  id: string
  title: string
  startedAt: number
}

/** 20 seconds of 16 kHz mono PCM16, the most one transcription request takes. */
const PIECE_SAMPLES = 320_000

const when = (ms: number) => new Date(ms).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })

/**
 * Meeting notes without a bot in the call: record the microphone here, the host transcribes it in
 * pieces, and a bot writes the minutes when asked. The recording never leaves this computer's host.
 */
export function MeetingsPanel({ store }: { store: BotStore }) {
  const tr = useTr()
  const [notes, setNotes] = useState<MeetingSummary[]>([])
  const [recording, setRecording] = useState<Recording | null>(null)
  const [title, setTitle] = useState('')
  const [open, setOpen] = useState<MeetingNote | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const stopMic = useRef<(() => void) | null>(null)
  const chunks = useRef<Int16Array[]>([])
  const samples = useRef(0)
  const sending = useRef<Promise<void>>(Promise.resolve())
  const session = useRef<string | null>(null)

  const load = async () => {
    try {
      const answer = await (await store.ready()).call<{ notes: MeetingSummary[] }>('meetingNotes', {})
      setNotes(answer.notes)
      setError(null)
    } catch (e) {
      setError(errorText(e))
    }
  }

  useEffect(() => {
    void load()
    return () => {
      stopMic.current?.()
      if (session.current) {
        void (async () => {
          try {
            await (await store.ready()).call('cancelMeeting', { id: session.current })
          } catch {
            // The host cleans a recording that was never finished.
          }
        })()
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store])

  const append = (pcm: Int16Array) => {
    chunks.current.push(pcm)
    samples.current += pcm.length
    if (samples.current < PIECE_SAMPLES) return
    flush()
  }

  const flush = () => {
    const take = chunks.current
    if (!take.length) return
    chunks.current = []
    samples.current = 0
    const bytes = new Uint8Array(take.reduce((n, chunk) => n + chunk.byteLength, 0))
    let at = 0
    for (const chunk of take) {
      bytes.set(new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength), at)
      at += chunk.byteLength
    }
    const id = session.current
    if (!id) return
    sending.current = sending.current
      .then(async () => {
        const client = await store.ready()
        await client.call('appendMeetingAudio', { id, audio: toBase64(bytes) }, 120_000)
      })
      .catch((e: unknown) => setError(errorText(e)))
  }

  const start = async () => {
    setBusy(true)
    setError(null)
    try {
      const answer = await (await store.ready()).call<{ id: string; title: string; startedAt: number }>(
        'startMeeting',
        { title: title.trim() },
      )
      session.current = answer.id
      chunks.current = []
      samples.current = 0
      sending.current = Promise.resolve()
      const stop = await micPCM16k((pcm) => append(pcm))
      stopMic.current = stop
      setRecording({ id: answer.id, title: answer.title, startedAt: answer.startedAt })
    } catch (e) {
      const id = session.current
      session.current = null
      if (id) {
        await (await store.ready()).call('cancelMeeting', { id }).catch(() => {})
      }
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const stop = async () => {
    if (!recording) return
    setBusy(true)
    stopMic.current?.()
    stopMic.current = null
    flush()
    try {
      await sending.current
      const client = await store.ready()
      const answer = await client.call<{ note: MeetingNote }>('finishMeeting', { id: recording.id }, 600_000)
      session.current = null
      setRecording(null)
      setOpen(answer.note)
      setTitle('')
      await load()
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const cancel = async () => {
    if (!recording) return
    stopMic.current?.()
    stopMic.current = null
    chunks.current = []
    samples.current = 0
    try {
      await (await store.ready()).call('cancelMeeting', { id: recording.id })
    } catch (e) {
      setError(errorText(e))
    }
    session.current = null
    setRecording(null)
  }

  const minutes = async (note: MeetingSummary) => {
    setBusy(true)
    try {
      const answer = await (await store.ready()).call<{ note: MeetingNote }>('meetingMinutes', { id: note.id }, 600_000)
      setOpen(answer.note)
      await load()
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const show = async (note: MeetingSummary) => {
    if (open?.id === note.id) {
      setOpen(null)
      return
    }
    try {
      const answer = await (await store.ready()).call<{ note: MeetingNote }>('meetingNote', { id: note.id })
      setOpen(answer.note)
      setError(null)
    } catch (e) {
      setError(errorText(e))
    }
  }

  const remove = async (note: MeetingSummary) => {
    try {
      await (await store.ready()).call('deleteMeetingNote', { id: note.id })
      if (open?.id === note.id) setOpen(null)
      await load()
    } catch (e) {
      setError(errorText(e))
    }
  }

  return (
    <section className="board-meetings" aria-label={tr('Meetings')}>
      <HStack gap={8} align="center">
        <Text size="callout" weight="semibold" grow>{tr('Meetings')}</Text>
        {busy ? <Spinner /> : null}
        <Text size="caption" tone="tertiary">{notes.length}</Text>
      </HStack>
      <Text size="caption" tone="tertiary">
        {tr('Record a meeting’s microphone here; the host writes it down and a bot can draft the minutes. Keep this window open while it records.')}
      </Text>

      {recording ? (
        <HStack gap={8} align="center" className="board-meeting-recording">
          <Icon name="waveform" size={16} color="var(--accent)" />
          <VStack gap={2} grow>
            <Text size="callout">{recording.title}</Text>
            <Text size="caption" tone="secondary">{tr('Recording since')} {when(recording.startedAt)}</Text>
          </VStack>
          <Button kind="secondary" disabled={busy} onClick={() => void stop()}>{tr('Stop and write it down')}</Button>
          <IconButton title={tr('Cancel')} icon="xmark" onClick={() => void cancel()} />
        </HStack>
      ) : (
        <HStack gap={8} align="end" className="board-meeting-add">
          <VStack gap={4} grow>
            <Text size="caption" tone="secondary">{tr('Meeting title')}</Text>
            <Field label="Meeting title" value={title} onChange={setTitle} placeholder="Weekly design review" />
          </VStack>
          <Button disabled={busy} onClick={() => void start()}>{tr('Record a meeting')}</Button>
        </HStack>
      )}

      {notes.map((note) => (
        <div key={note.id} className="board-meeting">
          <HStack gap={8} align="center">
            <VStack gap={2} grow>
              <Text size="callout">{note.title}</Text>
              <Text size="caption" tone="secondary">
                {when(note.startedAt)} · {note.chars.toLocaleString()} {tr('characters')}
                {note.hasMinutes ? ` · ${tr('Minutes')}` : ''}
              </Text>
            </VStack>
            {note.hasMinutes ? null : (
              <Button kind="secondary" disabled={busy} onClick={() => void minutes(note)}>{tr('Write minutes')}</Button>
            )}
            <IconButton title={open?.id === note.id ? tr('Close') : tr('Read')} icon={open?.id === note.id ? 'chevron.up' : 'chevron.down'} onClick={() => void show(note)} />
            <IconButton title={tr('Delete')} icon="trash" onClick={() => void remove(note)} />
          </HStack>
          {open?.id === note.id ? (
            <VStack gap={8} className="board-meeting-text">
              {open.minutes ? (
                <VStack gap={4}>
                  <Text size="caption" weight="semibold" tone="secondary">{tr('Minutes')}</Text>
                  <Text size="callout" className="board-meeting-body">{open.minutes}</Text>
                </VStack>
              ) : null}
              <VStack gap={4}>
                <Text size="caption" weight="semibold" tone="secondary">{tr('Transcript')}</Text>
                <Text size="callout" className="board-meeting-body">{open.transcript}</Text>
              </VStack>
            </VStack>
          ) : null}
        </div>
      ))}
      {notes.length === 0 ? <Text size="caption" tone="tertiary">{tr('No meeting notes yet.')}</Text> : null}
      {error ? <Text size="caption" tone="danger">{error}</Text> : null}
    </section>
  )
}
