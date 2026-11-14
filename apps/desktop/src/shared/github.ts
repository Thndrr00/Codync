/** The GitHub token of this computer's host, as `githubStatus` reports it, and its shape rules. */

export interface GitHubStatus {
  configured: boolean
  login: string | null
  scopes: string[]
  /** The token may create repositories (Administration, or a classic token's `repo` scope). */
  canCreateRepos: boolean
}

/** A repository `githubCreateRepo` just created; only a create answer carries it. */
export interface CreatedRepo {
  fullName: string
  htmlUrl: string
  cloneUrl: string
  /** The host pointed the workspace at it, because the workspace had no origin. */
  originSet: boolean
}

export const githubTokenOk = (token: string) =>
  /^[A-Za-z0-9_-]{20,255}$/.test(token.trim())

/** GitHub's own repository-name rule, checked before the host is asked. */
export const repoNameOk = (name: string) =>
  /^[A-Za-z0-9_-][A-Za-z0-9._-]{0,99}$/.test(name.trim())
