import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'

export interface DocumentSummary {
  id: string; botId: string; parentId: string | null; title: string
  revision: number; rev: number; createdAt: number; updatedAt: number
}
export interface Document extends DocumentSummary { body: string }
export interface DocumentDraft { id?: string; parentId?: string | null; title: string; body: string; ifRevision: number; clientNonce?: string }
export interface DocumentReview {
  id: string; botId: string; title: string; documentId?: string; status: string; createdAt: number; expiresAt: number
}
export interface ReviewDetail extends Omit<DocumentReview, 'title'> {
  draft: DocumentDraft; document: Document | null
}
export interface Activity {
  id: number; botId: string; action: string; actorKind: string; actorId: string
  result: 'success' | 'failed'; createdAt: number
}
type Saved = { status: 'saved' | 'conflict'; document: Document }

/** Host-owned documents and reviews. Failed saves keep the editable draft. */
export class DocumentsStore extends Observable {
  items: DocumentSummary[] = []
  reviews: DocumentReview[] = []
  activity: Activity[] = []
  review: ReviewDetail | null = null
  reviewCurrent: Document | null = null
  draft: DocumentDraft | null = null
  conflict: Document | null = null
  query = ''
  next: string | null = null
  loading = false
  opening = false
  extrasLoading = false
  busy = false
  mutating = false
  editorSession = 0
  dirty = false
  error: string | null = null
  notice: string | null = null
  draftWarning: string | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private active = true
  private retired = false
  private loadedQuery = ''
  private sequence = 0
  private listSequence = 0
  private extrasSequence = 0
  private drafts = new Map<string, DocumentDraft>()

  readonly botId: string
  private readonly client: () => Promise<HostClient>
  private readonly scope: string

  constructor(botId: string, client: () => Promise<HostClient>, scope: string) {
    super()
    this.botId = botId
    this.client = client
    this.scope = scope
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(this.storageKey) ?? '[]')
      if (Array.isArray(saved)) for (const entry of saved) {
        if (Array.isArray(entry) && typeof entry[0] === 'string' && isDraft(entry[1])) this.drafts.set(entry[0], entry[1])
      }
    } catch (error) { this.draftWarning = `Could not recover browser drafts: ${message(error)}` }
  }

  private get storageKey() { return `codync-documents-v1-${JSON.stringify([this.scope, this.botId])}` }
  get pendingDrafts() { return [...this.drafts].map(([key, draft]) => ({ key, title: draft.title || 'Untitled document' })) }
  private persistDrafts() {
    if (this.retired) return
    try { localStorage.setItem(this.storageKey, JSON.stringify([...this.drafts])); this.draftWarning = null }
    catch (error) { this.draftWarning = `Keep this panel open until saved. Browser draft backup failed: ${message(error)}` }
  }

  private async call<T>(method: string, body: object = {}) {
    if (this.retired) throw new Error('This computer connection was closed. Reopen Documents to continue.')
    return (await this.client()).call<T>(method, body)
  }

  async load(more = false, quiet = false) {
    const request = ++this.listSequence
    this.loading = !quiet
    if (!quiet) this.error = null
    this.changed()
    try {
      const query = this.query
      const count = query === this.loadedQuery ? Math.max(100, this.items.length) : 100
      const loaded: DocumentSummary[] = []
      let next = more ? this.next : null
      do {
        const page = await this.call<{ items: DocumentSummary[]; next: string | null }>('listDocuments', {
          botId: this.botId, query, ...(next ? { afterId: next } : {}), limit: 100,
        })
        if (request !== this.listSequence || this.retired) return
        loaded.push(...page.items)
        next = page.next
      } while (!more && loaded.length < count && next)
      if (request !== this.listSequence) return
      this.items = more ? [...this.items, ...loaded.filter((item) => !this.items.some((old) => old.id === item.id))] : loaded
      this.next = next
      this.loadedQuery = query
    } catch (error) { if (request === this.listSequence) this.error = message(error) }
    finally { if (request === this.listSequence) { this.loading = false; this.changed() } }
  }

  search(query: string) { this.query = query; void this.load() }

  async refreshExtras() {
    const request = ++this.extrasSequence
    this.extrasLoading = true
    this.changed()
    try {
      const [reviews, activity] = await Promise.all([
        this.call<{ items: DocumentReview[] }>('listDocumentReviews', { botId: this.botId }),
        this.call<{ items: Activity[] }>('listComputerActivity', { botId: this.botId }),
      ])
      if (request !== this.extrasSequence) return
      this.reviews = reviews.items
      this.activity = activity.items
    } catch (error) { if (request === this.extrasSequence) this.error = message(error) }
    if (request !== this.extrasSequence) return
    this.extrasLoading = false
    this.changed()
  }

  private retain() {
    this.cancelTimer()
    if (this.draft && this.dirty) this.drafts.set(this.draft.id ?? 'new', { ...this.draft })
    this.persistDrafts()
  }

  async open(id: string) {
    if (this.busy || this.retired) return
    ++this.editorSession
    this.retain()
    const request = ++this.sequence
    this.opening = true
    this.error = null
    this.conflict = null
    this.changed()
    try {
      const { document } = await this.call<{ document: Document }>('getDocument', { id })
      if (request !== this.sequence) return
      this.draft = this.drafts.get(id) ?? toDraft(document)
      this.dirty = this.drafts.has(id)
      if (this.dirty && this.draft.ifRevision !== document.revision) this.conflict = document
    } catch (error) { if (request === this.sequence) this.error = message(error) }
    finally { if (request === this.sequence) { this.opening = false; this.changed() } }
  }

  create(parentId: string | null = null) {
    if (this.busy || this.retired) return
    ++this.editorSession
    this.retain()
    ++this.sequence
    this.opening = false
    this.draft = this.drafts.get('new') ?? { title: '', body: '', parentId, ifRevision: 0, clientNonce: crypto.randomUUID() }
    this.draft.clientNonce ??= crypto.randomUUID()
    this.dirty = this.drafts.has('new')
    this.conflict = null
    this.error = null
    this.changed()
  }

  async recoverDraft(key: string) {
    const saved = this.drafts.get(key)
    if (!saved || this.busy) return
    this.retain(); ++this.sequence; ++this.editorSession
    this.draft = { ...saved }; this.dirty = true; this.conflict = null; this.error = null; this.opening = false
    this.changed()
    const session = this.editorSession
    if (saved.id) {
      try {
        const { document } = await this.call<{ document: Document }>('getDocument', { id: saved.id })
        if (session === this.editorSession && this.draft?.id === saved.id && this.draft.ifRevision === saved.ifRevision && saved.ifRevision !== document.revision) this.conflict = document
      } catch (error) { if (session === this.editorSession) this.error = message(error) }
    }
    this.changed()
  }

  discardDraft() {
    if (this.busy || !this.draft) return
    ++this.sequence; ++this.editorSession; this.opening = false
    this.cancelTimer(); this.drafts.delete(this.draft.id ?? 'new'); this.persistDrafts()
    this.draft = null; this.dirty = false; this.conflict = null; this.error = null; this.changed()
  }

  saveAsCopy() {
    if (this.busy || !this.draft) return
    if (this.draft.id && this.drafts.has('new')) { this.error = 'Finish or discard the existing new-document draft before creating another copy.'; this.changed(); return }
    this.cancelTimer(); this.drafts.delete(this.draft.id ?? 'new')
    this.draft = { title: this.draft.title, body: this.draft.body, ifRevision: 0, clientNonce: crypto.randomUUID() }
    this.conflict = null; this.dirty = true; this.drafts.set('new', { ...this.draft }); this.persistDrafts(); this.changed()
    void this.save()
  }

  edit(change: Partial<Pick<DocumentDraft, 'title' | 'body'>>) {
    if (!this.draft || this.mutating || this.retired) return
    this.draft = { ...this.draft, ...change }
    this.dirty = true
    this.drafts.set(this.draft.id ?? 'new', { ...this.draft })
    this.persistDrafts()
    this.notice = null
    this.cancelTimer()
    if (!this.conflict) this.timer = setTimeout(() => { void this.save() }, 800)
    this.changed()
  }

  async save(overwrite = false) {
    this.cancelTimer()
    if (!this.draft || this.busy || !this.dirty || !this.draft.title.trim() || (this.conflict && !overwrite)) return
    const submitted = { ...this.draft, ifRevision: overwrite && this.conflict ? this.conflict.revision : this.draft.ifRevision }
    const key = submitted.id ?? 'new'
    this.busy = true
    this.error = null
    this.changed()
    try {
      const { parentId, clientNonce, ...update } = submitted
      const result = await this.call<Saved>('saveDocument', { botId: this.botId, ...update, ...(!submitted.id ? { parentId, clientNonce } : {}) })
      if (this.retired) return
      if (result.status === 'conflict') { this.conflict = result.document; return }
      const changedDuringSave = this.draft.title.trim() !== result.document.title || this.draft.body !== result.document.body
      this.draft = { ...(changedDuringSave ? this.draft : toDraft(result.document)), id: result.document.id, ifRevision: result.document.revision }
      this.dirty = changedDuringSave
      this.conflict = null
      this.drafts.delete(key)
      if (this.dirty) this.drafts.set(result.document.id, { ...this.draft })
      this.persistDrafts()
      this.notice = this.dirty ? null : 'Saved'
      void this.load()
      this.cancelTimer()
      if (this.dirty && this.active) this.timer = setTimeout(() => { void this.save() }, 800)
    } catch (error) {
      this.error = message(error)
      if (!submitted.id) this.notice = 'Creation was not confirmed. Your draft is kept; retry Save to check the same request safely.'
    }
    finally { this.busy = false; this.changed() }
  }

  reloadConflict() {
    if (!this.conflict) return
    this.drafts.delete(this.conflict.id)
    this.persistDrafts()
    this.draft = toDraft(this.conflict)
    this.dirty = false
    this.conflict = null
    this.error = null
    this.changed()
  }

  async move(parentId: string | null) {
    if (!this.draft?.id || this.busy || this.dirty) return
    await this.changeDocument('moveDocument', { parentId })
  }

  async remove() {
    if (!this.draft?.id || this.busy) return
    await this.changeDocument('deleteDocument', {})
  }

  private async changeDocument(method: string, body: object) {
    if (!this.draft?.id) return
    const id = this.draft.id
    this.cancelTimer()
    this.busy = true
    this.mutating = true
    this.error = null
    this.changed()
    try {
      const result = await this.call<Saved | { status: 'deleted'; ids: string[] }>(method, { id, ifRevision: this.draft.ifRevision, ...body })
      if (result.status === 'conflict') this.conflict = result.document
      else if (result.status === 'deleted') {
        result.ids.forEach((deleted) => this.drafts.delete(deleted))
        this.persistDrafts()
        this.draft = null
        this.dirty = false
      } else this.draft = toDraft(result.document)
      void this.load()
    } catch (error) { this.error = message(error) }
    finally { this.busy = false; this.mutating = false; this.changed() }
  }

  async inspectReview(id: string) {
    if (this.busy) return
    this.busy = true
    this.error = null
    this.review = null
    this.reviewCurrent = null
    this.changed()
    try {
      const result = await this.call<{ review: ReviewDetail; currentDocument?: Document | null }>('getDocumentReview', { id })
      this.review = result.review; this.reviewCurrent = result.currentDocument ?? null
    }
    catch (error) { this.error = message(error) }
    finally { this.busy = false; this.changed() }
  }

  closeReview() { if (!this.busy) { this.review = null; this.changed() } }

  async resolve(decision: 'approve' | 'decline') {
    if (!this.review || this.busy || this.review.status !== 'pending') return
    this.busy = true
    this.error = null
    this.changed()
    try {
      this.review = (await this.call<{ review: ReviewDetail }>('resolveAction', { id: this.review.id, decision })).review
      await Promise.all([this.load(), this.refreshExtras()])
    } catch (error) { this.error = message(error) }
    finally { this.busy = false; this.changed() }
  }

  async copyActivity() {
    try {
      const { csv } = await this.call<{ csv: string }>('exportComputerActivity', { botId: this.botId })
      await navigator.clipboard.writeText(csv)
      this.notice = 'Activity CSV copied'
    } catch (error) { this.error = message(error) }
    this.changed()
  }

  resume() { this.active = true }
  dispose() { this.active = false; this.retain() }
  retire() { this.dispose(); this.retired = true; ++this.sequence; ++this.listSequence; ++this.extrasSequence }
  private cancelTimer() { if (this.timer) clearTimeout(this.timer); this.timer = null }
}

function toDraft(document: Document): DocumentDraft {
  return { id: document.id, title: document.title, body: document.body, ifRevision: document.revision }
}
function message(error: unknown) { return error instanceof Error ? error.message : String(error) }
function isDraft(value: unknown): value is DocumentDraft {
  if (!value || typeof value !== 'object') return false
  const draft = value as Record<string, unknown>
  return typeof draft.title === 'string' && typeof draft.body === 'string' && typeof draft.ifRevision === 'number'
    && (draft.id === undefined || typeof draft.id === 'string')
    && (draft.parentId === undefined || draft.parentId === null || typeof draft.parentId === 'string')
    && (draft.clientNonce === undefined || typeof draft.clientNonce === 'string')
}
