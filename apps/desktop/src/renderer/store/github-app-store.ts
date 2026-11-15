import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import type { GitHubAppStatus } from '../../shared/github'

/** The GitHub App of this computer's host: registering it, its installations, and which bot a repository reaches. */
export class GitHubAppStore extends Observable {
  status: GitHubAppStatus | null = null
  loading = false
  busy = false
  error: string | null = null
  private active = true
  private readonly client: () => Promise<Pick<HostClient, 'call'>>
  constructor(client: () => Promise<Pick<HostClient, 'call'>>) { super(); this.client = client }
  resume() { this.active = true }
  dispose() { this.active = false; this.loading = false; this.busy = false }

  async load() {
    if (!this.active || this.loading) return
    this.loading = true; this.changed()
    try {
      this.status = await (await this.client()).call<GitHubAppStatus>('githubAppStatus', {})
    } catch (error) { if (this.active) this.error = String(error) }
    finally { if (this.active) { this.loading = false; this.changed() } }
  }

  private async act(method: string, body: Record<string, unknown>) {
    if (!this.active || this.busy) return false
    this.busy = true; this.error = null; this.changed()
    try {
      this.status = await (await this.client()).call<GitHubAppStatus>(method, body)
      return true
    } catch (error) { if (this.active) this.error = String(error); return false }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  register(appId: string, privateKey: string, webhookSecret: string, slug: string) {
    return this.act('setGitHubApp', { appId, privateKey, webhookSecret, slug })
  }

  disconnect() { return this.act('disconnectGitHubApp', {}) }
  refresh() { return this.act('githubInstallations', {}) }
  map(repo: string, bots: string[], branches: string[]) { return this.act('mapGitHubRepo', { repo, bots, branches }) }
  unmap(repo: string) { return this.act('unmapGitHubRepo', { repo }) }
}
