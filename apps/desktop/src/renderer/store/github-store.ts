import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import type { CreatedRepo, GitHubStatus } from '../../shared/github'

/**
 * The GitHub token of this computer's host: its account, what it may do, and connecting it.
 * `created` is the repository the last create answered with, until another action replaces it.
 */
export class GitHubStore extends Observable {
  status: GitHubStatus | null = null
  created: CreatedRepo | null = null
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
      this.status = await (await this.client()).call<GitHubStatus>('githubStatus', {})
    } catch (error) { if (this.active) this.error = String(error) }
    finally { if (this.active) { this.loading = false; this.changed() } }
  }

  private async act(method: string, body: Record<string, unknown>) {
    if (!this.active || this.busy) return false
    this.busy = true; this.error = null; this.created = null; this.changed()
    try {
      const answer = await (await this.client()).call<GitHubStatus & { created?: CreatedRepo }>(method, body)
      this.status = answer
      this.created = answer.created ?? null
      return true
    } catch (error) { if (this.active) this.error = String(error); return false }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  connect(token: string) { return this.act('setGitHubToken', { token }) }
  disconnect() { return this.act('disconnectGitHub', {}) }
  createRepo(name: string, isPrivate: boolean, description: string) {
    return this.act('githubCreateRepo', { name: name.trim(), private: isPrivate, description: description.trim() })
  }
}
