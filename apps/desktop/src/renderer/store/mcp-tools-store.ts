import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'

export type ToolMode = 'allow' | 'ask' | 'deny'
export interface ToolPolicy { tool: string; mode: ToolMode; explicit?: boolean }
export interface ToolRequest {
  id: string; botId: string; tool: string; status: string; createdAt: number; expiresAt: number
}
export interface ToolDetail extends ToolRequest { arguments: unknown }
type Page = { items: ToolRequest[]; next: string | null }

/** Only transient management state lives here; authorization always belongs to the host. */
export class McpToolsStore extends Observable {
  policies: ToolPolicy[] = []
  requests: ToolRequest[] = []
  detail: ToolDetail | null = null
  next: string | null = null
  loading = false
  inspecting = false
  busy = false
  error: string | null = null
  notice: string | null = null
  receipt: { id: string; status: string; reply: unknown } | null = null
  private sequence = 0
  private inspection = 0
  private disposed = false

  readonly connectorId: string
  private readonly client: () => Promise<Pick<HostClient, 'call'>>
  constructor(connectorId: string, client: () => Promise<Pick<HostClient, 'call'>>) {
    super(); this.connectorId = connectorId; this.client = client
  }
  private async call<T>(method: string, body: object = {}): Promise<T> {
    return (await this.client()).call<T>(method, method === 'resolveToolCall' ? body : { connectorId: this.connectorId, ...body })
  }
  resume() { this.disposed = false }
  async load(more = false) {
    if (this.disposed || this.busy || this.loading || (more && !this.next)) return
    const sequence = ++this.sequence
    const visibleCount = this.requests.length
    this.loading = true; this.changed()
    try {
      const [settings, firstPage] = await Promise.all([
        this.call<{ items: ToolPolicy[] }>('getMcpToolSettings'),
        this.call<Page>('listMcpToolCalls', more ? { afterId: this.next } : {}),
      ])
      if (sequence !== this.sequence || this.disposed) return
      let page = firstPage
      const rows = [...page.items]
      while (!more && rows.length < visibleCount && page.next) {
        page = await this.call<Page>('listMcpToolCalls', { afterId: page.next })
        if (sequence !== this.sequence || this.disposed) return
        rows.push(...page.items)
      }
      this.policies = settings.items
      this.requests = [...new Map([...(more ? this.requests : []), ...rows].map((row) => [row.id, row])).values()]
      this.next = page.next; this.error = null
      if (this.detail) {
        const current = this.requests.find((row) => row.id === this.detail?.id)
        this.detail = current ? { ...this.detail, ...current } : null
      }
    } catch (error) {
      if (sequence === this.sequence && !this.disposed) this.error = String(error)
    } finally {
      if (sequence === this.sequence && !this.disposed) { this.loading = false; this.changed() }
    }
  }
  async inspect(row: ToolRequest) {
    const sequence = ++this.inspection
    this.detail = null; this.inspecting = true; this.error = null; this.changed()
    try {
      const detail = await this.call<Omit<ToolDetail, 'createdAt' | 'expiresAt'>>('getMcpToolCall', { id: row.id })
      if (sequence === this.inspection && !this.disposed) this.detail = { ...row, ...detail }
    } catch (error) {
      if (sequence === this.inspection && !this.disposed) this.error = String(error)
    } finally {
      if (sequence === this.inspection && !this.disposed) { this.inspecting = false; this.changed() }
    }
  }
  closeDetail() { ++this.inspection; this.detail = null; this.inspecting = false; this.changed() }
  private async mutate(method: string, body: object, notice: string) {
    if (this.busy || this.disposed) return
    const sequence = ++this.sequence
    this.loading = false; this.busy = true; this.error = null; this.notice = null; this.changed()
    try {
      await this.call(method, body)
      if (this.disposed || sequence !== this.sequence) return
      this.closeDetail(); this.notice = notice
    } catch (error) {
      if (!this.disposed && sequence === this.sequence) this.error = String(error)
    } finally {
      if (!this.disposed && sequence === this.sequence) { this.busy = false; this.changed() }
    }
    // Keep an action failure visible while refreshing only after successful changes.
    if (!this.error && !this.disposed && sequence === this.sequence) await this.load()
  }
  setPolicy(tool: string, mode: ToolMode) {
    return this.mutate('setMcpToolPolicy', { tool, mode }, 'Policy saved. Earlier authorizations for this tool were revoked.')
  }
  resetPolicy(tool: string) {
    return this.mutate('resetMcpToolPolicy', { tool }, 'Server default restored. Earlier authorizations were revoked.')
  }
  decide(id: string, approve: boolean) {
    // The host accepts only a decision, never replacement arguments.
    return this.mutate('resolveToolCall', { id, approve }, approve ? 'Decision recorded. The waiting proxy may run this request once.' : 'Request declined.')
  }
  revokeAll() {
    return this.mutate('revokeMcpToolCalls', {}, 'Current requests revoked. Future requests still follow their policies.')
  }
  async retrieveResult(id: string) {
    if (this.disposed) return
    const sequence = ++this.inspection
    this.receipt = null; this.inspecting = true; this.error = null; this.changed()
    try {
      const receipt = await this.call<{ status: string; reply: unknown }>('getMcpToolResult', { id })
      if (!this.disposed && sequence === this.inspection) this.receipt = { ...receipt, id }
    } catch (error) { if (!this.disposed && sequence === this.inspection) this.error = String(error) }
    finally { if (!this.disposed && sequence === this.inspection) { this.inspecting = false; this.changed() } }
  }
  dispose() {
    this.disposed = true; ++this.sequence; ++this.inspection
    this.loading = false; this.inspecting = false; this.busy = false
  }
}
