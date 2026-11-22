import type { MediaKind } from '@shared/media'
import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import {
  clampCoordinate, type FlowCanvas, type FlowLink, type FlowMedia,
  type WorkflowFolderAction, type WorkflowFoldersSnapshot,
} from '../lib/routine-flow.ts'

/** What a view needs after a move: the node's new place, shown at once and saved on the host. */
type Client = () => Promise<Pick<HostClient, 'call'>>

/**
 * One bot's routine-flow canvas. The host owns the flow (rules, runs, steps); this keeps what is
 * loaded, the request in flight and the last error. A change reloads the canvas, and so does a
 * `routineFlow` event (the store's `flowRev` for the bot moving).
 */
export class RoutineFlowStore extends Observable {
  canvas: FlowCanvas | null = null
  loading = false
  busy = false
  error: string | null = null
  /** Set when the host has no flow methods (an older host): the canvas hides. */
  unsupported = false
  /** The computer-wide "All workflows" folder tree; null until it has been read. */
  folders: WorkflowFoldersSnapshot | null = null
  foldersLoading = false
  /** A folder call's own error: folders are additive, so a refusal never hides the canvas. */
  foldersError: string | null = null
  private sequence = 0
  private disposed = false

  botId: string
  private readonly client: Client

  constructor(botId: string, client: Client) {
    super()
    this.botId = botId
    this.client = client
  }

  /** A view mounted again (React may mount twice): the store works again. */
  resume() {
    this.disposed = false
  }

  /**
   * The tab strip changed workflow. The canvas is dropped first, so the view shows its loading
   * spinner rather than the previous workflow's steps while the new one is read.
   */
  switchTo(botId: string) {
    if (botId === this.botId) return
    ++this.sequence
    this.botId = botId
    this.canvas = null
    this.loading = false
    this.error = null
    this.unsupported = false
    this.changed()
    void this.load()
  }

  dispose() {
    this.disposed = true
  }

  private fail(error: unknown) {
    const text = error instanceof Error ? error.message : String(error)
    if (/unknown method|404/i.test(text)) this.unsupported = true
    this.error = text
  }

  async load() {
    if (this.disposed) return
    const sequence = ++this.sequence
    this.loading = true
    this.changed()
    try {
      const canvas = await (await this.client()).call<FlowCanvas>('routineFlowCanvas', { botId: this.botId })
      if (sequence !== this.sequence || this.disposed) return
      this.canvas = canvas
      this.error = null
      this.unsupported = false
    } catch (error) {
      if (sequence === this.sequence && !this.disposed) this.fail(error)
    } finally {
      if (sequence === this.sequence && !this.disposed) {
        this.loading = false
        this.changed()
      }
    }
  }

  /** A change the user made: runs it, then loads the canvas again. Whether it worked. */
  private async change<T>(method: string, body: object): Promise<T | null> {
    if (this.disposed) return null
    this.busy = true
    this.error = null
    this.changed()
    try {
      const answer = await (await this.client()).call<T>(method, body)
      await this.load()
      return answer
    } catch (error) {
      this.fail(error)
      return null
    } finally {
      this.busy = false
      this.changed()
    }
  }

  /** A node moved: shown where it was dropped at once, then saved. */
  async move(nodeKey: string, x: number, y: number) {
    const place = { nodeKey, x: clampCoordinate(x), y: clampCoordinate(y) }
    if (this.canvas) {
      this.canvas = { ...this.canvas, positions: [...this.canvas.positions.filter((p) => p.nodeKey !== nodeKey), place] }
      this.changed()
    }
    await this.change('saveRoutineFlowPosition', { botId: this.botId, ...place })
  }

  async place(placedBotId: string, x: number, y: number) {
    await this.change('placeRoutineFlowBot', { botId: this.botId, placedBotId, x: clampCoordinate(x), y: clampCoordinate(y) })
  }

  async removeNode(nodeKey: string) {
    await this.change('removeRoutineFlowPosition', { botId: this.botId, nodeKey })
  }

  async connect(routineId: string, fromBotId: string, toBotId: string, instruction: string): Promise<FlowLink | null> {
    const answer = await this.change<{ link: FlowLink }>('connectRoutineFlow', { routineId, fromBotId, toBotId, instruction })
    return answer?.link ?? null
  }

  /** Saves the instruction and the model the step runs on; a null model is the next bot's own. */
  async updateLink(linkId: string, instruction: string, model: string | null, modelOptions: Record<string, string>) {
    await this.change('updateRoutineFlowLink', { linkId, instruction, model, modelOptions })
  }

  async disconnect(linkId: string) {
    await this.change('disconnectRoutineFlow', { linkId })
  }

  /** Gives a bot or media step a name; an empty one falls back to the step type. */
  async renameStep(routineId: string, nodeKey: string, name: string) {
    await this.change('renameRoutineFlowStep', { botId: this.botId, routineId, nodeKey, name })
  }

  /** Adds a picture or clip step to a routine, at its model's first settings. */
  async addMedia(routineId: string, kind: MediaKind): Promise<FlowMedia | null> {
    const answer = await this.change<{ media: FlowMedia }>('addRoutineFlowMedia', { routineId, kind })
    return answer?.media ?? null
  }

  /** Saves a step's description, model and settings. */
  async updateMedia(mediaId: string, settings: object) {
    await this.change('updateRoutineFlowMedia', { mediaId, ...settings })
  }

  async removeMedia(mediaId: string) {
    await this.change('removeRoutineFlowMedia', { mediaId })
  }

  /** Starts the flow with a sample reply, without running the routine. */
  async test(routineId: string) {
    await this.change('testRoutineFlow', { botId: this.botId, routineId })
  }

  /** Pauses or resumes a routine from the inspector. */
  async setEnabled(routineId: string, enabled: boolean) {
    await this.change('setRoutineEnabled', { botId: this.botId, id: routineId, enabled })
  }

  /** The routines (the host's workflows) of these bots, for the All workflows menu. */
  async listWorkflows(botIds: readonly string[]): Promise<{ id: string; name: string; botId: string }[]> {
    const client = await this.client()
    const lists = await Promise.all(botIds.map((botId) =>
      client.call<{ routines: { id: string; name: string }[] }>('routines', { botId })))
    return lists.flatMap((list, index) => list.routines.map((routine) => ({ ...routine, botId: botIds[index] })))
  }

  /** The computer's workflow folders and where each workflow sits. */
  async loadFolders() {
    if (this.disposed) return
    this.foldersLoading = true
    this.foldersError = null
    this.changed()
    try {
      this.folders = await (await this.client()).call<WorkflowFoldersSnapshot>('workflowFolders', {})
    } catch (error) {
      this.foldersError = error instanceof Error ? error.message : String(error)
    } finally {
      this.foldersLoading = false
      this.changed()
    }
  }

  /** Creates, renames, deletes or files a workflow; the host returns the new snapshot. */
  async folderAction(action: WorkflowFolderAction) {
    if (this.disposed) return
    this.busy = true
    this.foldersError = null
    this.changed()
    try {
      this.folders = await (await this.client()).call<WorkflowFoldersSnapshot>('workflowFolderApply', action)
    } catch (error) {
      this.foldersError = error instanceof Error ? error.message : String(error)
    } finally {
      this.busy = false
      this.changed()
    }
  }
}
