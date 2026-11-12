import { Observable } from '../../lib/observable.ts'

/** Device-only call state shared with the island; audio stays with CallView's engine. */
export interface ActiveCallSnapshot {
  id: string
  ref: string
  name: string
  phase: string
  muted: boolean
}

type CallAction = 'mute' | 'end'
type CallActions = Record<CallAction, () => void>

class ActiveCall extends Observable {
  snapshot: ActiveCallSnapshot | null = null
  private registration: { actions: CallActions } | null = null

  /** A previous effect's cleanup cannot erase a newer call registration. */
  publish(snapshot: ActiveCallSnapshot, actions: CallActions) {
    const registration = { actions }
    this.registration = registration
    this.snapshot = { ...snapshot }
    this.changed()
    return () => {
      if (this.registration !== registration) return
      this.registration = null
      this.snapshot = null
      this.changed()
    }
  }

  invoke(action: CallAction) {
    const registration = this.registration
    if (!registration || !this.snapshot) return false
    registration.actions[action]()
    return true
  }
}

export const activeCall = new ActiveCall()
