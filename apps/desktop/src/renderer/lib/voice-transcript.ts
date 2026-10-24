export type TranscriptSource = 'recognized' | 'generated' | 'queued'
export interface TranscriptEntry { role: 'user' | 'assistant'; source: TranscriptSource; text: string }
export interface VoiceTranscript { entries: TranscriptEntry[]; truncated: boolean; interruptions: number }

/** Observed call text, never proof of playback or an approved decision. */
export class CallTranscript {
  private rows = new Map<string, TranscriptEntry>()
  private characters = 0
  private sequence = 0
  private geminiTurn = 0
  private truncated = false
  private interruptions = 0

  add(role: TranscriptEntry['role'], source: TranscriptSource, text: string, id = `local-${this.sequence++}`, delta = false) {
    if (!text.length || id.length > 256) return
    const captured = text.replaceAll('\0', '')
    if (captured.length !== text.length) this.truncated = true
    if (!captured.length) return
    const old = this.rows.get(id)
    if (!old && this.rows.size >= 96) { this.truncated = true; return }
    if (old && (old.role !== role || old.source !== source)) return
    const available = Math.min(4000, 32000 - this.characters + (old?.text.length ?? 0))
    const full = (delta ? old?.text ?? '' : '') + captured
    let next = full.slice(0, available)
    if (/[\uD800-\uDBFF]$/.test(next)) next = next.slice(0, -1)
    if (next.length < full.length) this.truncated = true
    if (!next.trim()) return
    this.characters += next.length - (old?.text.length ?? 0)
    this.rows.set(id, { role, source, text: next })
  }

  interrupted() { this.interruptions = Math.min(10000, this.interruptions + 1) }

  openAI(event: Record<string, unknown>) {
    if (typeof event.item_id !== 'string' || typeof event.content_index !== 'number') return
    const id = `openai-${event.item_id}-${event.content_index}`
    if (typeof event.transcript !== 'string') return
    if (event.type === 'conversation.item.input_audio_transcription.completed') this.add('user', 'recognized', event.transcript, id)
    if (event.type === 'response.output_audio_transcript.done') this.add('assistant', 'generated', event.transcript, id)
  }

  gemini(content: Record<string, unknown>) {
    for (const [field, role] of [['inputTranscription', 'user'], ['outputTranscription', 'assistant']] as const) {
      const value = content[field] as { text?: unknown } | undefined
      if (typeof value?.text === 'string') this.add(role, role === 'user' ? 'recognized' : 'generated', value.text, `gemini-${this.geminiTurn}-${role}`, true)
    }
    if (content.interrupted === true) this.interrupted()
    if (content.turnComplete === true) this.geminiTurn++
  }

  snapshot(): VoiceTranscript {
    return { entries: [...this.rows.values()].map((entry) => ({ ...entry })), truncated: this.truncated, interruptions: this.interruptions }
  }
}
