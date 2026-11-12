import type { Backend, Entry, SharedFile } from '../../shared/models'
import type { BotApp } from '../../shared/bdots-models'
import { Observable } from '../lib/observable.ts'

/** A destination stays available until its selected conversation has mounted. */
export type IslandThreadDestination = (
  | { kind: 'budget'; ref: string }
  | { kind: 'miniApp'; ref: string; app: BotApp }
  | { kind: 'operationPanel'; ref: string; panel: 'trace' | 'computer' | 'documents'; documentId?: string }
  | { kind: 'memory'; ref: string }
  | { kind: 'terminal'; ref: string; id: string; backend: Backend | null; step: 'install' | 'login' | 'node' }
  | { kind: 'artifact'; ref: string; artifactId: string }
  | { kind: 'file'; ref: string; entryId: string; file: SharedFile; entry: Entry }
) & { owner?: object }

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
  consume(ref: string, owner?: object): PendingDestination | null {
    const destination = this.pending
    if (!destination || destination.ref !== ref) return null
    if (destination.owner && destination.owner !== owner) { this.pending = null; this.changed(); return null }
    this.pending = null
    this.changed()
    return destination
  }
}

export const islandThreadNavigation = new IslandThreadNavigation()
