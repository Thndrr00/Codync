import type { Backend, Entry, SharedFile } from '../../shared/models'
import { Observable } from '../lib/observable.ts'

/** A destination stays available until its selected conversation has mounted. */
export type IslandThreadDestination =
  | { kind: 'memory'; ref: string }
  | { kind: 'terminal'; ref: string; id: string; backend: Backend | null; step: 'install' | 'login' | 'node' }
  | { kind: 'artifact'; ref: string; artifactId: string }
  | { kind: 'file'; ref: string; entryId: string; file: SharedFile; entry: Entry }

type PendingDestination = IslandThreadDestination & { sequence: number }

class IslandThreadNavigation extends Observable {
  pending: PendingDestination | null = null
  private sequence = 0

  open(destination: IslandThreadDestination) {
    const sequence = ++this.sequence
    this.pending = { ...destination, sequence }
    this.changed()
    return sequence
  }

  /** Mounting or rerendering another conversation must not discard the request. */
  consume(ref: string): PendingDestination | null {
    const destination = this.pending
    if (!destination || destination.ref !== ref) return null
    this.pending = null
    this.changed()
    return destination
  }
}

export const islandThreadNavigation = new IslandThreadNavigation()
