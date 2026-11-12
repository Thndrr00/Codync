/** Bounded presentation data. Tokens, terminal output, memory bodies and audio stay with their owners. */
export interface IslandGitHub {
  configured: boolean
  repository: string | null
  branch: string | null
  fetchedAt?: number
  error: string | null
  pulls: { number: number; title: string; state: string; draft: boolean; updatedAt: string; url: string; reviewers: string[]; reviews: { login: string; state: string }[]; checks: { name: string; status: string; conclusion: string | null; url: string | null }[] }[]
  issues: { number: number; title: string; state: string; assignees: string[]; updatedAt: string; url: string }[]
}

export interface IslandIntegrations {
  ref: string
  loading: boolean
  busy: boolean
  fetchedAt: number | null
  errors: { section: string; message: string }[]
  actionError: string | null
  savedNote: string | null
  github: IslandGitHub | null
  terminals: { id: string; backend: string; step: 'install' | 'login' | 'node'; exitCode: number | null }[]
  memories: { file: string; shared: boolean; description: string | null; confirmed: string | null }[]
  connectors: { id: string; name: string; auth: string; failure: string | null }[]
  artifacts: { id: string; title: string; updatedAt: number; revised: boolean }[]
  delegations: { id: string; from: string; to: string; at: number }[]
  upcoming: { id: string; name: string; at: number }[]
  bookmarks: { id: string; entryId: string; threadId: string | null; title: string }[]
  searches: { id: string; query: string; scope: 'all' | 'main' | 'threads' }[]
  search: { id: string; loading: boolean; error: string | null; hits: { id: string; botId: string; threadId: string | null; text: string }[] } | null
}

export interface IslandSignals {
  computers: { id: string; name: string; state: string; error: string | null }[]
  uploads: { ref: string; name: string; percent: number | null; status: string }[]
  dictation: { id: string; phase: 'starting' | 'listening' | 'transcribing'; startedAt: number | null } | null
}

export type IslandIntegrationAction =
  | { kind: 'loadIntegrations'; ref: string }
  | { kind: 'reconnectComputer'; ref: string }
  | { kind: 'openMemory'; ref: string }
  | { kind: 'openConnector'; ref: string; id: string }
  | { kind: 'openAgentTerminal'; ref: string; id: string }
  | { kind: 'openIntegrationArtifact'; ref: string; id: string }
  | { kind: 'openGitHubLink'; ref: string; url: string }
  | { kind: 'appendScratchpad'; ref: string; text: string; requestId: string }
  | { kind: 'runSavedSearch'; ref: string; id: string }
  | { kind: 'openBookmark'; ref: string; id: string }
  | { kind: 'openSearchHit'; ref: string; id: string }
  | { kind: 'stopDictation'; id: string }

/** Unknown/future start times stay unknown rather than displaying a misleading age. */
export function islandApprovalAge(createdAt: number | undefined, now: number): number | null {
  return typeof createdAt === 'number' && Number.isFinite(createdAt) && Number.isFinite(now) && createdAt > 0 && createdAt <= now ? Math.floor((now - createdAt) / 60_000) : null
}

export function projectIslandGitHub(value: IslandGitHub): IslandGitHub {
  return {
    configured: value.configured, repository: value.repository, branch: value.branch, error: value.error, fetchedAt: value.fetchedAt,
    pulls: value.pulls.slice(0, 2).map((pull) => ({ number: pull.number, title: pull.title, state: pull.state, draft: pull.draft, updatedAt: pull.updatedAt, url: pull.url, reviewers: pull.reviewers.slice(0, 30), reviews: pull.reviews.slice(0, 100).map((review) => ({ login: review.login, state: review.state })), checks: pull.checks.slice(0, 40).map((check) => ({ name: check.name, status: check.status, conclusion: check.conclusion, url: check.url })) })),
    issues: value.issues.slice(0, 10).map((issue) => ({ number: issue.number, title: issue.title, state: issue.state, assignees: issue.assignees.slice(0, 30), updatedAt: issue.updatedAt, url: issue.url })),
  }
}

export function safeIslandGitHubUrl(value: unknown): value is string {
  if (typeof value !== 'string' || value.length > 2048) return false
  try { const url = new URL(value); return url.protocol === 'https:' && url.hostname === 'github.com' && !url.port && !url.username && !url.password }
  catch { return false }
}
