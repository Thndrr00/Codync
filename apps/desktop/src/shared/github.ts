/** The GitHub token of this computer's host, as `githubStatus` reports it, and its shape rules. */

export interface GitHubStatus {
  configured: boolean
  login: string | null
  scopes: string[]
}

export const githubTokenOk = (token: string) =>
  /^[A-Za-z0-9_-]{20,255}$/.test(token.trim())

/** One App installation and the repositories it may reach. */
export interface GitHubInstallation {
  id: string
  account: string
  repos: string[]
}

/** A repository that reaches bots, on the branches listed (none means all of them). */
export interface GitHubRepoMap {
  repo: string
  bots: string[]
  branches: string[]
}

/** The GitHub App of this computer's host, as `githubAppStatus` reports it. */
export interface GitHubAppStatus {
  configured: boolean
  appId?: string
  slug?: string
  installations: GitHubInstallation[]
  repos: GitHubRepoMap[]
  events?: string[]
  tokenExpiresAt?: number | null
}

export const githubAppIdOk = (id: string) => /^\d{1,20}$/.test(id.trim())
export const githubWebhookSecretOk = (secret: string) => /^[\x21-\x7e]{16,200}$/.test(secret.trim())
export const githubRepoOk = (repo: string) => /^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(repo.trim()) && repo.trim().length <= 140
