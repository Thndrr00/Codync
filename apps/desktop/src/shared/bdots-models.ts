// Wire types for bdots-inspired features: mini-apps, capability links, open items,
// proactive settings, action class policies, robot avatar looks and MCP failure diagnostics.

export type ActionClass =
  | 'files'
  | 'shell'
  | 'browse'
  | 'browser'
  | 'apps'
  | 'bots'
  | 'routines'

export type RulePolicy = 'allow' | 'ask' | 'block'

export const ACTION_CLASSES: {
  id: ActionClass
  label: string
  hint: string
}[] = [
  { id: 'files', label: 'Files', hint: 'Read, write or edit workspace files' },
  { id: 'shell', label: 'Shell', hint: 'Run commands and background jobs' },
  { id: 'browse', label: 'Web fetch', hint: 'Fetch public URLs and search' },
  { id: 'browser', label: 'Browser control', hint: 'Drive pages and click controls' },
  { id: 'apps', label: 'Connected apps', hint: 'Call MCP connectors and secrets' },
  { id: 'bots', label: 'Team bots', hint: 'Delegate to or message other bots' },
  { id: 'routines', label: 'Routines', hint: 'Create or modify scheduled routines' },
]

export type RobotLookId =
  | 'round'
  | 'visor'
  | 'dome'
  | 'boxy'
  | 'cyclops'
  | 'ears'
  | 'scout'
  | 'owl'
  | 'pixel'
  | 'astro'
  | 'cat'
  | 'hex'

export interface BotAvatarLook {
  id: RobotLookId
  label: string
}

export const BOT_AVATAR_LOOKS: BotAvatarLook[] = [
  { id: 'round', label: 'Round' },
  { id: 'visor', label: 'Visor' },
  { id: 'dome', label: 'Dome' },
  { id: 'boxy', label: 'Boxy' },
  { id: 'cyclops', label: 'Cyclops' },
  { id: 'ears', label: 'Ears' },
  { id: 'scout', label: 'Scout' },
  { id: 'owl', label: 'Owl' },
  { id: 'pixel', label: 'Pixel' },
  { id: 'astro', label: 'Astro' },
  { id: 'cat', label: 'Cat' },
  { id: 'hex', label: 'Hex' },
]

export interface ProactiveSettings {
  enabled: boolean
  everyHours: number
  quietStart: number
  quietEnd: number
  timezone?: string | null
  maxPerDay: number
  appsInBackground: boolean
}

export const DEFAULT_PROACTIVE_SETTINGS: ProactiveSettings = {
  enabled: false,
  everyHours: 4,
  quietStart: 22,
  quietEnd: 8,
  timezone: null,
  maxPerDay: 3,
  appsInBackground: true,
}

export function defaultProactiveSettings(): ProactiveSettings {
  return { ...DEFAULT_PROACTIVE_SETTINGS }
}

export type AppLinkMode = 'view' | 'edit'

export interface AppLink {
  token: string
  botId: string
  slug: string
  label: string
  mode: AppLinkMode
  url?: string
  createdAt: number
  lastOpenedAt?: number | null
  opens: number
}

/** A newly issued capability and its complete host-provided URL. */
export interface CreatedAppLink {
  link: AppLink
  url: string
}

export interface BotApp {
  slug: string
  botId: string
  title: string
  description?: string | null
  entry: string
  version: number
  createdAt: number
  updatedAt: number
  ownerToken?: string
  url: string
  links?: AppLink[]
  collections?: { collection: string; count: number; updatedAt: number }[]
}

export interface OpenItem {
  id: string
  botId: string
  title: string
  note?: string
  kind?: string
  detail?: string | null
  dueAt?: number | null
  sourceUrl?: string | null
  createdAt: number
  updatedAt: number
  checkAfter?: number | null
  status: 'open' | 'resolved' | 'dismissed'
}

export interface ConnectorFailure {
  kind: 'auth' | 'unreachable' | 'tool'
  message: string
  at: number
}

/** Canonicalizes a project workspace path so Windows/Unix separator & trailing-slash variants match (N16). */
export function canonicalWorkspacePath(path: string): string {
  const normalized = path.trim().replace(/\\/g, '/').replace(/\/+$/, '')
  if (!normalized) return ''
  return /^[a-zA-Z]:/.test(normalized) ? normalized.toLowerCase() : normalized
}

/**
 * Returns other active bots working in the same project workspace as `bot` (N16: simultaneous project work warning).
 * Managed personal workspaces (`managedWorkspace === true` or empty `cwd`) and group chats are excluded.
 */
export function sharedWorkspacePeers<
  T extends { id: string; kind?: string; cwd: string; managedWorkspace?: boolean; status: string; deleted?: boolean },
>(bot: T, roster: Iterable<T>): T[] {
  if (bot.kind === 'group' || bot.managedWorkspace) return []
  const target = canonicalWorkspacePath(bot.cwd)
  if (!target) return []
  const out: T[] = []
  for (const peer of roster) {
    if (
      peer.id !== bot.id &&
      !peer.deleted &&
      peer.kind !== 'group' &&
      !peer.managedWorkspace &&
      (peer.status === 'working' || peer.status === 'needsInput') &&
      canonicalWorkspacePath(peer.cwd) === target
    ) {
      out.push(peer)
    }
  }
  return out
}

export interface MessageBookmark {
  id: string
  botId: string
  entryId: string
  threadId?: string | null
  author: string
  preview: string
  note: string
  createdAt: number
  updatedAt: number
}

export type PendingDecisionKind = 'permission' | 'passkey' | 'incident'

export interface PendingDecision {
  entryId: string
  botId: string
  kind: PendingDecisionKind
  title: string
  detail: string
  createdAt: number
}

/** Collects unresolved permissions, passkeys, and error incidents across chat entries (Ficha N12). */
export function collectPendingDecisions(
  entries: Iterable<{
    id: string
    botId: string
    kind: string
    createdAt: number
    data: {
      status?: string
      title?: string
      text?: string
      domain?: string
      error?: string
    }
  }>,
): PendingDecision[] {
  const out: PendingDecision[] = []
  for (const e of entries) {
    if (e.kind === 'permission' && (e.data.status ?? 'pending') === 'pending') {
      out.push({
        entryId: e.id,
        botId: e.botId,
        kind: 'permission',
        title: e.data.title || 'Tool permission required',
        detail: e.data.text || '',
        createdAt: e.createdAt,
      })
    } else if (e.kind === 'passkey' && (e.data.status ?? 'pending') === 'pending') {
      out.push({
        entryId: e.id,
        botId: e.botId,
        kind: 'passkey',
        title: e.data.domain ? `Passkey for ${e.data.domain}` : 'Passkey verification required',
        detail: e.data.text || '',
        createdAt: e.createdAt,
      })
    } else if (e.kind === 'error' && (e.data.error || e.data.text)) {
      out.push({
        entryId: e.id,
        botId: e.botId,
        kind: 'incident',
        title: e.data.title || 'Agent incident',
        detail: e.data.error || e.data.text || '',
        createdAt: e.createdAt,
      })
    }
  }
  return out.sort((a, b) => b.createdAt - a.createdAt)
}

export interface DeliverableItem {
  fileId: string
  entryId: string
  botId: string
  name: string
  size: number
  sha256: string
  createdAt: number
}

/** Extracts shared files and generated deliverables from chat entries (Ficha N09). */
export function collectDeliverables(
  entries: Iterable<{
    id: string
    botId: string
    createdAt: number
    data: {
      files?: { id: string; name: string; size: number; sha256: string }[]
    }
  }>,
): DeliverableItem[] {
  const seen = new Set<string>()
  const out: DeliverableItem[] = []
  for (const e of entries) {
    const files = e.data.files
    if (!files?.length) continue
    for (const f of files) {
      if (!f.id || seen.has(f.id)) continue
      seen.add(f.id)
      out.push({
        fileId: f.id,
        entryId: e.id,
        botId: e.botId,
        name: f.name,
        size: f.size,
        sha256: f.sha256,
        createdAt: e.createdAt,
      })
    }
  }
  return out.sort((a, b) => b.createdAt - a.createdAt)
}

export interface WorkerRoleTemplate {
  id: 'assistant' | 'reviewer' | 'coordinator' | 'researcher' | 'qa'
  label: string
  summary: string
  standingInstructions: string
  avatarLook: RobotLookId
  avatarColor: string
  actionRules: Partial<Record<ActionClass, RulePolicy>>
}

/** Specialized worker role profiles with preset standing instructions, avatar look & autonomy policies (Ficha N15). */
export const WORKER_ROLE_TEMPLATES: WorkerRoleTemplate[] = [
  {
    id: 'assistant',
    label: 'General Assistant',
    summary: 'Balanced day-to-day coding and operational partner',
    standingInstructions:
      'Act as a pragmatic software partner. Explain key tradeoffs concisely, verify changes with tests, and keep the user informed of blockers.',
    avatarLook: 'round',
    avatarColor: 'blue',
    actionRules: { files: 'allow', shell: 'ask', browse: 'allow', bots: 'allow' },
  },
  {
    id: 'reviewer',
    label: 'Code Reviewer',
    summary: 'Read-first auditor focused on regressions, security and edge cases',
    standingInstructions:
      'Focus on code review, correctness, security boundaries, and missing tests. Do not mutate files or run destructive commands unless explicitly asked.',
    avatarLook: 'owl',
    avatarColor: 'amber',
    actionRules: { files: 'ask', shell: 'ask', browse: 'allow', browser: 'block' },
  },
  {
    id: 'coordinator',
    label: 'Team Coordinator',
    summary: 'Delegates tasks across specialist bots and tracks open items',
    standingInstructions:
      'Break complex goals into clear subtasks, delegate to specialist team bots, track open items until resolved, and synthesize concise status updates.',
    avatarLook: 'visor',
    avatarColor: 'purple',
    actionRules: { bots: 'allow', routines: 'allow', apps: 'allow', shell: 'ask' },
  },
  {
    id: 'researcher',
    label: 'Deep Researcher',
    summary: 'Investigates documentation, web sources and codebase architecture',
    standingInstructions:
      'Gather primary sources, cross-check claims against code and documentation, cite concrete files and URLs, and organize findings into structured briefs.',
    avatarLook: 'scout',
    avatarColor: 'teal',
    actionRules: { browse: 'allow', files: 'allow', shell: 'block', browser: 'allow' },
  },
  {
    id: 'qa',
    label: 'QA Verifier',
    summary: 'Runs test suites, reproduces bugs and validates user flows',
    standingInstructions:
      'Reproduce reported issues systematically, run unit and end-to-end checks, inspect logs, and report pass/fail evidence clearly.',
    avatarLook: 'boxy',
    avatarColor: 'green',
    actionRules: { files: 'allow', shell: 'allow', browser: 'allow', browse: 'allow' },
  },
]

const FILLER_PATTERN = /^(?:um+|uh+|eh+|erm+|este+|pues|bueno|like)\b[,.\s]*/i
const INLINE_FILLER_PATTERN = /\b(?:um+|uh+|eh+|erm+)\b[,.\s]*/gi

/**
 * Local, deterministic voice dictation polisher inspired by Coucou (`investigacion-coucou-integracion-codync.md`):
 * strips leading/inline filler vocalizations, normalizes spacing around punctuation, and capitalizes sentences.
 */
export function polishVoiceDictation(raw: string): string {
  let text = raw.trim()
  if (!text) return ''
  text = text.replace(FILLER_PATTERN, '').replace(INLINE_FILLER_PATTERN, ' ')
  text = text
    .replace(/\s+([,.;:!?])/g, '$1')
    .replace(/([,.;:!?])([^\s,.;:!?])/g, '$1 $2')
    .replace(/\s{2,}/g, ' ')
    .trim()
  if (!text) return ''
  text = text.charAt(0).toUpperCase() + text.slice(1)
  text = text.replace(/([.!?]\s+)([a-zñáéíóúü])/g, (_, sep: string, ch: string) => sep + ch.toUpperCase())
  return text
}

