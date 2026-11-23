import type { MediaKind } from '@shared/media'
import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import {
  clampCoordinate, parseWorkflowFile, type FlowCanvas, type FlowLink, type FlowMedia,
  type WorkflowFileInfo, type WorkflowFolderAction, type WorkflowFoldersSnapshot,
} from '../lib/routine-flow.ts'

/** What a view needs after a move: the node's new place, shown at once and saved on the host. */
type Client = () => Promise<Pick<HostClient, 'call'>>

/** A workflow file the host made, or the steps that must lose their secrets first. */
export type WorkflowExport = { name: string; content: string } | { secrets: string[] }

/** What a workflow file holds, before the person maps its agent and channel names. */
export interface WorkflowImportInfo extends WorkflowFileInfo {
  /** Connections and media accounts left out of the file, shown as "step: app". */
  links: { step: string; app: string }[]
}

/** The fixed body of `importRoutineFlow` with the person's name choices. */
export interface WorkflowImportBody {
  name: string
  content: string
  agents: Record<string, string>
  channels: Record<string, string>
}

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

  /**
   * Tries the workflow method name this client was given first, then the older host's name.
   * Both are live paths: a host on the new protocol answers the first, an older one the second.
   */
  private async firstAnswer<T>(calls: readonly { method: string; body: object }[], map: (answer: unknown) => T | null): Promise<T | null> {
    if (this.disposed) return null
    this.busy = true
    this.error = null
    this.changed()
    try {
      for (const call of calls) {
        try {
          const value = map(await (await this.client()).call<unknown>(call.method, call.body))
          if (value !== null) return value
        } catch (error) {
          const text = error instanceof Error ? error.message : String(error)
          if (!/unknown method|404|not found/i.test(text)) {
            this.error = text
            return null
          }
        }
      }
      return null
    } finally {
      this.busy = false
      this.changed()
    }
  }

  /** A workflow change: the first method that answers wins, then the canvas is read again. */
  private async firstChange<T>(calls: readonly { method: string; body: object }[], map: (answer: unknown) => T | null): Promise<T | null> {
    const value = await this.firstAnswer(calls, map)
    if (value !== null) await this.load()
    return value
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

  /** Saves a step's "name: type" answer fields and its check; an empty nodeKey is the trigger. */
  async setAnswers(routineId: string, nodeKey: string, answer: string, check: string) {
    await this.change('setRoutineFlowAnswers', { botId: this.botId, routineId, nodeKey, answer, check })
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

  /**
   * Pauses or resumes a whole workflow: its triggers stop starting runs, a manual run still works.
   * The fixed body is `{ routineId, paused }`; the older host's two methods take `{botId, routineId}`.
   */
  async setPaused(routineId: string, paused: boolean) {
    await this.firstChange<boolean>([
      { method: 'setRoutineFlowPaused', body: { routineId, paused } },
      { method: paused ? 'pauseRoutineWorkflow' : 'resumeRoutineWorkflow', body: { botId: this.botId, routineId } },
    ], (answer) => answer && typeof answer === 'object' ? true : null)
  }

  /** Makes a workflow file; the host answers with its bytes or names the steps holding a secret. */
  async exportFlow(routineId: string): Promise<WorkflowExport | null> {
    const file = await this.firstAnswer<WorkflowExport>([
      { method: 'exportRoutineFlow', body: { routineId } },
      { method: 'exportRoutineWorkflow', body: { botId: this.botId, routineId } },
    ], (answer) => {
      const value = answer as { name?: string; content?: string; secrets?: string[]; fileName?: string; file?: string }
      if (Array.isArray(value?.secrets)) return { secrets: value.secrets }
      if (typeof value?.content === 'string') return { name: value.name ?? '', content: value.content }
      if (typeof value?.file === 'string') return { name: value.fileName ?? '', content: value.file }
      return null
    })
    if (file) return file
    // The older host refuses a secret with a message that names the steps to change.
    const message = this.error ?? ''
    if (/secret in a step/i.test(message)) {
      const steps = (message.split('Change the secret in:')[1] ?? '').replace(/\.$/, '')
      return { secrets: steps.split(',').map((step) => step.trim()).filter(Boolean) }
    }
    return null
  }

  /** Reads a file's name and the agent/channel names to choose before an import, on this computer. */
  async inspectImport(content: string): Promise<WorkflowImportInfo | null> {
    const info = parseWorkflowFile(content)
    return info ? { ...info, links: [] } : null
  }

  /** Imports the file with the person's name choices; the host adds it paused. */
  async importFlow(body: WorkflowImportBody): Promise<boolean> {
    const names = Object.fromEntries(Object.entries({ ...body.agents, ...body.channels }).filter(([, id]) => id))
    return (await this.firstChange<boolean>([
      { method: 'importRoutineFlow', body: { action: 'apply', ...body } },
      { method: 'importRoutineWorkflow', body: { botId: this.botId, name: body.name, file: body.content, names } },
    ], (answer) => answer && typeof answer === 'object' ? true : null)) === true
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
