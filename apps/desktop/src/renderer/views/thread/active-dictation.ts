import { Observable } from '../../lib/observable.ts'

export interface ActiveDictationSnapshot {
  id: string
  phase: 'starting' | 'listening' | 'transcribing'
  startedAt: number | null
}

/** Window-local presentation only. No audio, recognized text or provider credentials leave the owner. */
class ActiveDictation extends Observable {
  snapshot: ActiveDictationSnapshot | null = null

  publish(snapshot: ActiveDictationSnapshot) {
    if (this.snapshot?.id === snapshot.id && this.snapshot.phase === snapshot.phase && this.snapshot.startedAt === snapshot.startedAt) return
    this.snapshot = { ...snapshot }
    this.changed()
  }

  /** An old recognizer finishing must never clear its replacement's presentation. */
  clear(id: string) {
    if (this.snapshot?.id !== id) return
    this.snapshot = null
    this.changed()
  }
}

export const activeDictation = new ActiveDictation()
