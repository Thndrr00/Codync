import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable'
import type { Belief } from '../../shared/beliefs'
import type { Brain } from '../../shared/models'

/** One memory entry (the host's `codync_mem::store::Entry`). */
export interface MemoryEntry {
  id: number
  /** The bot whose own layer holds it; null for what all bots share. */
  bot: string | null
  state: 'active' | 'archived' | string
  text: string
  /** user · bot · dream · imported */
  author: string
  authorBot: string | null
  pinned: boolean
  evidence: string[]
  createdAt: number
  updatedAt: number
}

export interface MemoryLayer { entries: MemoryEntry[]; used: number; cap: number }

/** One dreaming run: what it added and archived (`undone` when an older app undid it). */
export interface DreamRun {
  id: number
  at: number
  undone: boolean
  added: { id: number; text: string }[]
  archived: { id: number; text: string }[]
  /** Long-form files it added a line to. */
  files?: { name: string; shared: boolean; version: number }[]
}

/** A long-form memory file as the host last recorded it (`codync_mem::files::File`). */
export interface MemoryFile {
  bot: string | null
  name: string
  description: string | null
  tokens: number
  deleted: boolean
  updatedAt: number
}

export interface MemoryFiles {
  yours: { files: MemoryFile[]; folder: string } | null
  bot: { files: MemoryFile[]; folder: string }
  /** Tokens one file may hold, and files one folder may hold. */
  cap: number
  folderCap: number
}

/** A file as it is on disk now. */
export interface MemoryFileDetail {
  file: string
  shared: boolean
  content: string | null
  path: string
}

export interface MemoryView {
  /** Whether this bot uses a brain to share memory (the Fleet one included). */
  shared: boolean
  /** The brain it works in; null when it works in none. */
  brain: Brain | null
  /** The brain's memory ("About you" in the Fleet brain, the project's otherwise); null without a brain. */
  yours: MemoryLayer | null
  bot: MemoryLayer
  /** Tokens used against the two caps together. */
  total: { used: number; cap: number }
  dreams: DreamRun[]
  /** Claude Code's own notes for the bot's project folder, read-only. */
  projectMemory: { path: string; text: string } | null
  memoryRev: number
  /** The long-form files of both layers. */
  files?: MemoryFiles
}

const OLDER_HOST = "Update Codync on your computer to see this bot's memory."

/** One bot's memory, read-only: the bot changes it. */
export class MemoryStore extends Observable {
  view: MemoryView | null = null
  error: string | null = null
  /** Every brain with the bots in it; null until read. */
  brains: Brain[] | null = null
  /** The notes as beliefs (`beliefs`); null until read. */
  beliefs: Belief[] | null = null
  beliefError: string | null = null
  private showingForgotten = false
  private request = 0

  constructor(readonly botId: string, private readonly client: () => Promise<HostClient>) { super() }

  async load() {
    const request = ++this.request
    try {
      const view = await (await this.client()).call<MemoryView>('memory', { botId: this.botId })
      // A host before 2.18.0 answers with its old memory listing, which has no layers or dreams.
      if (!Array.isArray(view.dreams)) throw new Error(OLDER_HOST)
      if (request === this.request) {
        this.view = view
        this.error = null
      }
    } catch (error) {
      if (request === this.request) this.error = message(error)
    }
    this.changed()
  }

  /** Every brain, and the one the bot is in after a change to it. */
  async loadBrains() {
    try {
      const reply = await (await this.client()).call<{ brains: Brain[] }>('brains', {})
      this.brains = reply.brains
      this.error = null
    } catch (error) {
      this.error = message(error)
    }
    this.changed()
  }

  /** Makes a brain; with `id`, renames that one. */
  async saveBrain(name: string, id?: string) {
    try {
      const reply = await (await this.client()).call<{ brains: Brain[] }>(id ? 'renameBrain' : 'createBrain', { id, name })
      this.brains = reply.brains
      this.error = null
      await this.load()
    } catch (error) {
      this.error = message(error)
    }
    this.changed()
  }

  /** Moves the bot into `brain` (the Fleet one is the shared memory). */
  async joinBrain(brain: Brain) {
    try {
      const reply = await (await this.client()).call<{ brains: Brain[] }>('joinBrain', { botId: this.botId, brainId: brain.id })
      this.brains = reply.brains
      this.error = null
      await this.load()
    } catch (error) {
      this.error = message(error)
    }
    this.changed()
  }

  /** Takes the bot out of its brain; it keeps only its own memory (picking one again brings sharing back). */
  async leaveBrain() {
    try {
      const reply = await (await this.client()).call<{ brains: Brain[] }>('leaveBrain', { botId: this.botId })
      this.brains = reply.brains
      this.error = null
      await this.load()
    } catch (error) {
      this.error = message(error)
    }
    this.changed()
  }

  /** Deletes a brain; its bots go back to the Fleet brain and what it held is gone. */
  async deleteBrain(brain: Brain) {
    try {
      const reply = await (await this.client()).call<{ brains: Brain[] }>('deleteBrain', { id: brain.id })
      this.brains = reply.brains
      this.error = null
      await this.load()
    } catch (error) {
      this.error = message(error)
    }
    this.changed()
  }

  /** A brain as one JSON file, or the brain made from such a file. */
  async exportBrain(brain: Brain): Promise<string> {
    return JSON.stringify(await (await this.client()).call('exportBrain', { id: brain.id }), null, 2)
  }

  async importBrain(bundle: string, name?: string): Promise<number> {
    const reply = await (await this.client()).call<{ brains: Brain[]; skipped: unknown[] }>('importBrain', {
      bundle: JSON.parse(bundle), name,
    })
    this.brains = reply.brains
    await this.load()
    this.changed()
    return reply.skipped.length
  }

  /** The bot's notes as beliefs, or with `forgotten` the ones that were forgotten. */
  async loadBeliefs(forgotten = this.showingForgotten) {
    this.showingForgotten = forgotten
    try {
      const reply = await (await this.client()).call<{ beliefs: Belief[] }>('beliefs', { botId: this.botId, forgotten })
      if (this.showingForgotten === forgotten) { this.beliefs = reply.beliefs; this.beliefError = null }
    } catch (error) {
      this.beliefError = message(error)
    }
    this.changed()
  }

  /** Confirms a note, forgets it, or brings an earlier version back; the list is read again after. */
  async beliefAction(belief: Belief, action: 'confirm' | 'forget' | 'restore', versionId?: number) {
    try {
      await (await this.client()).call('beliefAction', { botId: this.botId, file: belief.file, shared: belief.shared, action, versionId })
      await this.loadBeliefs()
    } catch (error) {
      this.beliefError = message(error)
      this.changed()
    }
  }

  /** Drops the error of an action that failed once the sheet has read the latest; what the user wrote stays. */
  dismissError() {
    this.error = null
    this.changed()
  }

  /** A long-form file as it is now, with its versions. */
  async file(name: string, shared: boolean): Promise<MemoryFileDetail | null> {
    try {
      return await (await this.client()).call<MemoryFileDetail>('memoryFile', { botId: this.botId, file: name, shared })
    } catch (error) {
      this.error = message(error)
      this.changed()
      return null
    }
  }
}

/** Who wrote an entry, for its detail line. */
export function authorLabel(entry: MemoryEntry, botName: (id: string | null) => string) {
  switch (entry.author) {
    case 'user': return 'You wrote it'
    case 'bot': return `${botName(entry.authorBot)} remembered it`
    case 'dream': return 'Dreaming'
    default: return 'Moved from the old memory'
  }
}

function message(error: unknown) { return error instanceof Error ? error.message : String(error) }
