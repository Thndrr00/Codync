import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
export type ComputerPermissions = { browser: boolean; files: boolean; shell: boolean }
export interface ComputerPanelState {
  enabled: boolean; permissions: ComputerPermissions; workspace: string
  screen: { userControl: boolean; agentBot?: string; computerUse: boolean; protectedEntry?: { id: string; botId: string; site: string } | null }
  activity: { id: number; action: string; result: string; createdAt: number }[]
  services?: { research: string; slack: string }
  protectedBrowserAvailable?: boolean
}
export interface ComputerFile { name: string; directory: boolean; link: boolean }

export class ComputerPanelStore extends Observable {
  state: ComputerPanelState | null = null
  files: ComputerFile[] = []
  path = ''
  fileText: string | null = null
  output: { stdout: string; stderr: string; exitCode: number | null; success: boolean } | null = null
  truncated = false
  loading = false
  busy = false
  running = false
  error: string | null = null
  notice: string | null = null
  private alive = true
  private sequence = 0
  private browse = 0
  private lifetime = 0
  private readonly botId: string
  private readonly client: () => Promise<Pick<HostClient, 'call'>>
  constructor(botId: string, client: () => Promise<Pick<HostClient, 'call'>>) { super(); this.botId = botId; this.client = client }
  private async call<T>(method: string, body: object = {}, timeout = 20_000) {
    return (await this.client()).call<T>(method, { botId: this.botId, ...body }, timeout)
  }
  resume() { this.alive = true }
  dispose() { this.alive = false; ++this.sequence; ++this.browse; ++this.lifetime; this.loading = false; this.busy = false; this.running = false }
  async load(clearError = false, duringMutation = false) {
    if (!this.alive || this.loading || (this.busy && !duringMutation)) return
    const sequence = ++this.sequence
    if (clearError) this.error = null
    this.loading = true; this.changed()
    try {
      const state = await this.call<ComputerPanelState>('computerPanel')
      if (!this.alive || sequence !== this.sequence) return
      this.state = state
      if (!state.enabled || !state.permissions.files) { ++this.browse; this.files = []; this.fileText = null }
    } catch (error) { if (this.alive && sequence === this.sequence) this.error = String(error) }
    finally { if (this.alive && sequence === this.sequence) { this.loading = false; this.changed() } }
  }
  async setPermissions(enabled: boolean, permissions: ComputerPermissions) {
    if (this.busy || !this.alive) return
    const lifetime = this.lifetime
    ++this.sequence; this.loading = false
    this.busy = true; this.error = null; this.changed()
    try {
      await (await this.client()).call('updateBot', { id: this.botId, computer: enabled, computerPermissions: permissions })
      if (this.alive && lifetime === this.lifetime) await this.load(false, true)
    } catch (error) { if (this.alive && lifetime === this.lifetime) this.error = String(error) }
    finally { if (this.alive && lifetime === this.lifetime) { this.busy = false; this.changed() } }
  }
  async open(path: string, directory = true) {
    if (!this.alive) return
    const sequence = ++this.browse
    this.fileText = null; this.error = null; this.changed()
    try {
      if (directory) {
        const page = await this.call<{ items: ComputerFile[]; truncated: boolean }>('listComputerFiles', { path })
        if (!this.alive || sequence !== this.browse) return
        this.files = page.items; this.path = path; this.truncated = page.truncated
      } else {
        const file = await this.call<{ text: string }>('readComputerFile', { path })
        if (!this.alive || sequence !== this.browse) return
        this.fileText = file.text
      }
    } catch (error) { if (this.alive && sequence === this.browse) this.error = String(error) }
    finally { if (this.alive && sequence === this.browse) this.changed() }
  }
  async run(command: string) {
    if (!this.alive || this.running) return
    const lifetime = this.lifetime
    this.running = true; this.output = null; this.error = null; this.changed()
    try { const output = await this.call<NonNullable<ComputerPanelStore['output']>>('runComputerCommand', { command }, 40_000); if (this.alive && lifetime === this.lifetime) this.output = output }
    catch (error) { if (this.alive && lifetime === this.lifetime) this.error = String(error) }
    finally { if (this.alive && lifetime === this.lifetime) { this.running = false; this.changed(); await this.load() } }
  }
  async protectedEntry(site: string | null, id?: string, keepSession = false) {
    if (this.busy || !this.alive) return
    const lifetime = this.lifetime
    ++this.sequence; this.loading = false
    this.busy = true; this.error = null; this.changed()
    try {
      const client = await this.client()
      if (site !== null) await client.call('beginProtectedEntry', { botId: this.botId, site })
      else {
        const result = await client.call<{browserSession?:string}>('finishProtectedEntry', { id, ...(keepSession ? { keepSession: true } : {}) })
        if (this.alive && lifetime === this.lifetime) this.notice = result.browserSession === 'preserved' ? 'Private sign-in session preserved in a new blank window. All previous private pages were closed.' : result.browserSession === 'closed' ? 'Protected browser closed. Its private sign-in session was discarded.' : 'Protected entry finished.'
      }
      if (this.alive && lifetime === this.lifetime) await this.load(false, true)
    } catch (error) { if (this.alive && lifetime === this.lifetime) this.error = String(error) }
    finally { if (this.alive && lifetime === this.lifetime) { this.busy = false; this.changed() } }
  }
  async protectedBrowser(method: 'openProtectedBrowser' | 'fillProtectedLogin' | 'fillProtectedCode', id: string, value?: string) {
    if (this.busy || !this.alive) return
    const lifetime = this.lifetime
    this.busy = true; this.error = null; this.changed()
    try {
      const payload = { id, ...(method === 'fillProtectedLogin' ? { loginId: value } : method === 'fillProtectedCode' ? { code: value } : {}) }
      await (await this.client()).call(method, payload, 30000)
    } catch (error) { if (this.alive && lifetime === this.lifetime) this.error = String(error) }
    finally { if (this.alive && lifetime === this.lifetime) { this.busy = false; this.changed() } }
  }
}
