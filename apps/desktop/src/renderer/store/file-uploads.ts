import type { HostClient } from '../client/host-client'
import type { OutgoingFile } from './outgoing-files.ts'

/** How a file attached to the message box is getting to the computer. */
export type UploadStatus = 'uploading' | 'uploaded' | 'failed' | 'cancelled'

export interface UploadState {
  /** The id the computer knows this upload by (`cancelUpload`, then `send`). */
  uploadId: string
  status: UploadStatus
  /** Bytes the computer has, and the file's size. */
  sent: number
  total: number
  /** Why it failed, as the computer said it. */
  error?: string
}

type Link = <T>(op: (client: HostClient) => Promise<T>) => Promise<T>
type Waiter = { resolve: (uploadId: string) => void; reject: (error: Error) => void }

/**
 * The uploads of the files in a chat's message box. Each file starts uploading when it is attached,
 * so its ring shows what the computer has before the message is sent; a click on the ring cancels
 * it, and a failed file stays in the box with its reason until it is retried or removed.
 *
 * A file uploads under its own id, apart from the pick it came with: one failure or cancel never
 * takes the others down, and a retry sends only that file again.
 */
export class FileUploads {
  private readonly states = new Map<string, UploadState>()
  /** The run under way for each file, which a cancel marks before the chunks stop. */
  private readonly runs = new Map<string, { cancelled: boolean }>()
  private readonly botIds = new Map<string, string>()
  private readonly waiting = new Map<string, Waiter[]>()

  constructor(
    private readonly withLink: Link,
    private readonly changed: () => void,
    /** The store's own percentages, which the Dynamic Island reads (null once a file is done). */
    private readonly percents: Map<string, number>,
  ) {}

  state(fileId: string): UploadState | undefined {
    return this.states.get(fileId)
  }

  /** A file just attached: the computer starts receiving it now. */
  begin(botId: string, file: OutgoingFile) {
    const status = this.states.get(file.id)?.status
    if (status === 'uploading' || status === 'uploaded') return
    this.start(botId, file)
  }

  /** Starts a failed or cancelled file again, dropping the partial it left on the computer. */
  retry(botId: string, file: OutgoingFile) {
    const previous = this.states.get(file.id)
    if (previous && previous.status !== 'uploaded') {
      void this.withLink((client) => client.cancelUpload(botId, previous.uploadId)).catch(() => {})
    }
    const run = this.runs.get(file.id)
    if (run) run.cancelled = true
    this.start(botId, file)
  }

  /** Stops a file's upload: the computer drops its partial chunks, and the file waits in the box. */
  cancel(fileId: string) {
    const state = this.states.get(fileId)
    const botId = this.botIds.get(fileId)
    if (!state || !botId || state.status !== 'uploading') return
    const run = this.runs.get(fileId)
    if (run) run.cancelled = true
    this.runs.delete(fileId)
    this.set(fileId, { ...state, status: 'cancelled', sent: 0 })
    void this.withLink((client) => client.cancelUpload(botId, state.uploadId)).catch(() => {})
  }

  /** The file left the box: its upload stops, and what the computer has of it goes too. */
  forget(fileId: string) {
    const state = this.states.get(fileId)
    const botId = this.botIds.get(fileId)
    const run = this.runs.get(fileId)
    if (run) run.cancelled = true
    this.runs.delete(fileId)
    this.botIds.delete(fileId)
    this.states.delete(fileId)
    this.percents.delete(fileId)
    if (state) this.percents.delete(state.uploadId)
    this.waiting.delete(fileId)
    if (state?.status === 'uploading' && botId) {
      void this.withLink((client) => client.cancelUpload(botId, state.uploadId)).catch(() => {})
    }
    this.changed()
  }

  /** The message carrying them arrived: their rings are done with. */
  release(files: OutgoingFile[]) {
    for (const file of files) {
      const state = this.states.get(file.id)
      this.states.delete(file.id)
      this.percents.delete(file.id)
      if (state) this.percents.delete(state.uploadId)
      this.botIds.delete(file.id)
    }
  }

  /**
   * Waits until every file is uploaded, then names their upload ids in order. A file that failed or
   * was cancelled is started again, so sending waits for it as the box says; its error rejects.
   */
  ids(botId: string, files: OutgoingFile[]): Promise<string[]> {
    return Promise.all(files.map((file) => this.idOf(botId, file)))
  }

  private idOf(botId: string, file: OutgoingFile): Promise<string> {
    const state = this.states.get(file.id)
    if (state?.status === 'uploaded') return Promise.resolve(state.uploadId)
    return new Promise<string>((resolve, reject) => {
      this.waiting.set(file.id, [...(this.waiting.get(file.id) ?? []), { resolve, reject }])
      if (!state || state.status === 'failed' || state.status === 'cancelled' || state.status === 'uploading') {
        this.begin(botId, file)
      }
    })
  }

  private start(botId: string, file: OutgoingFile) {
    const uploadId = crypto.randomUUID()
    const run = { cancelled: false }
    this.botIds.set(file.id, botId)
    this.runs.set(file.id, run)
    this.set(file.id, { uploadId, status: 'uploading', sent: 0, total: file.data.length })
    void (async () => {
      try {
        await this.withLink((client) =>
          client.upload(
            botId,
            uploadId,
            file.name,
            file.data,
            (sent) => {
              if (run.cancelled) return
              const current = this.states.get(file.id)
              if (current?.uploadId !== uploadId || current.status !== 'uploading' || current.sent === sent) return
              this.set(file.id, { ...current, sent })
            },
            () => run.cancelled,
          ),
        )
        if (run.cancelled || this.states.get(file.id)?.uploadId !== uploadId) return
        this.set(file.id, { uploadId, status: 'uploaded', sent: file.data.length, total: file.data.length })
      } catch (error) {
        if (run.cancelled) {
          // A chunk already on its way may have put the partial back: drop it again now that it stopped.
          void this.withLink((client) => client.cancelUpload(botId, uploadId)).catch(() => {})
          return
        }
        const current = this.states.get(file.id)
        if (current && current.uploadId !== uploadId) return
        const sent = current?.sent ?? 0
        this.set(file.id, {
          uploadId,
          status: 'failed',
          sent,
          total: file.data.length,
          error: error instanceof Error ? error.message : String(error),
        })
      } finally {
        if (this.runs.get(file.id) === run) this.runs.delete(file.id)
      }
    })()
  }

  private set(fileId: string, state: UploadState) {
    this.states.set(fileId, state)
    this.percents.delete(fileId)
    this.percents.delete(state.uploadId)
    if (state.status === 'uploading') {
      const percent = Math.min(99, Math.floor((state.sent * 100) / Math.max(1, state.total)))
      // The chat rows name a sent attachment by its upload id, which is what the island reads.
      this.percents.set(fileId, percent)
      this.percents.set(state.uploadId, percent)
    }
    this.changed()
    const waiters = this.waiting.get(fileId)
    if (!waiters) return
    if (state.status === 'uploaded') {
      this.waiting.delete(fileId)
      for (const waiter of waiters) waiter.resolve(state.uploadId)
    } else if (state.status === 'failed' || state.status === 'cancelled') {
      this.waiting.delete(fileId)
      const error = new Error(state.error ?? 'The upload was cancelled')
      for (const waiter of waiters) waiter.reject(error)
    }
  }
}
