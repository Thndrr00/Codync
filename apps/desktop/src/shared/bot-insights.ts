// The read-only bdots-inspired insights a host serves for one bot. The host computes every value;
// clients only render it.

export interface InsightNotify {
  headline: string
  tone: string
  state: string
  stateIcon: string
  marker: string
  diagnostic: number
  freeform: number
  delivered: number
  failed: number
  muted: boolean
  maxOutput: number
  maxAttempts: number
  floodDefault: number
  floodCap: number
  retryDelay: number
  obligation: string
  summary: { key: string; value: string }[]
}

export interface InsightMarkdown {
  headline: string
  tone: string
  preview: string
  blocks: number
  links: number
  mentions: number
  wide: boolean
  heading: string
  summary: { key: string; value: string }[]
}

export interface InsightLocalFmt {
  headline: string
  tone: string
  locale: string
  clock: string
  day: string
  weekday: string
  shortDate: string
  month: string
  relative: string
  timeAgo: string
  count: string
  tokens: string
  spend: string
  budget: string
  ordinal: string
  summary: { key: string; value: string }[]
}

export interface InsightTextFmt {
  headline: string
  tone: string
  slug: string
  handle: string
  excerpt: string
  validEmails: number
  invalidEmails: number
  hidden: number
  passwordLike: number
  typoFix: string
  summary: { key: string; value: string }[]
}

export interface InsightAgentesChat {
  headline: string
  window: string
  windowIcon: string
  windowTone: string
  used: string
  percent: number
  auto: string
  autoTone: string
  autoEfforts: string[]
  timeout: number
  placeKept: number
  groupWindow: number
  summary: { key: string; value: string }[]
}

export interface InsightOpenBot {
  headline: string
  tone: string
  engine: string
  model: string
  effort: string
  effortIcon: string
  effortTone: string
  route: string
  routeIcon: string
  plan: boolean
  thread: string
  responsesApi: boolean
  summary: { key: string; value: string }[]
}

export interface InsightOpenDots {
  headline: string
  container: string
  valid: boolean
  delivery: boolean
  deliveryError: string
  learningIcon: string
  learningTone: string
  internal: number
  scheduledLabel: string
  voiceLabel: string
  summary: { key: string; value: string }[]
}

export interface InsightContext {
  headline: string
  page: string
  screenshots: string
  pageKeep: number
  screenshotKeep: number
  staleMin: number
  pageNote: string
  screenshotNote: string
  pruned: string
  tone: string
  summary: { key: string; value: string }[]
}

export interface InsightLiveness {
  headline: string
  tone: string
  state: string
  icon: string
  active: boolean
  age: string
  generation: number
  heartbeat: number
  due: boolean
  outcome: string
  outcomeIcon: string
  outcomeTone: string
  summary: { key: string; value: string }[]
}

export interface InsightDictation {
  headline: string
  tone: string
  engine: string
  engineIcon: string
  engineTone: string
  model: string
  language: string
  supported: boolean
  needsCommit: boolean
  commitMs: number
  count: string
  summary: { key: string; value: string }[]
}

export interface InsightAccess {
  mode: string
  modeIcon: string
  modeTone: string
  canWrite: boolean
  form: boolean
  count: string
  collections: { name: string; valid: boolean }[]
  invalid: string[]
  routing: string
  routingIcon: string
  routingTone: string
  liveName: string
  summary: { key: string; value: string }[]
}

export interface InsightBlocks {
  headline: string
  count: string
  said: number
  work: number
  workOnly: boolean
  quiet: boolean
  origin: string
  originIcon: string
  summary: { key: string; value: string }[]
  blocks: { kind: string; text: string; icon: string; tone: string; says: boolean }[]
}

export interface InsightDirectory {
  headline: string
  tone: string
  total: string
  ready: number
  unread: number
  busy: number
  percent: number
  bar: string
  directory: string
  email: string
  routable: boolean
  routeIcon: string
  routeTone: string
  reason: string
  summary: { key: string; value: string }[]
  integrations: { id: string; name: string; endpoint: boolean; header: string }[]
}

export interface InsightPrompts {
  model: string
  context: string
  price: string
  valid: boolean
  checkIn: string
  zone: string
  clock: string
  watch: string
  length: string
  instruction: string
  summary: { key: string; value: string }[]
  lines: string[]
}

export interface InsightFresh {
  agent: string
  deployment: string
  language: string
  receipt: string
  profile: string
  summary: { key: string; value: string }[]
  titles: { exec: string; write: string }
  resources: { path: string; kind: string; script: boolean; reference: boolean; asset: boolean }[]
}

export interface InsightAgentProfile {
  headline: string
  label: string
  tone: string
  score: number
  account: string
  accountIcon: string
  accountTone: string
  mode: string
  modeIcon: string
  modeTone: string
  fast: boolean
  fastIcon: string
  fastTone: string
  catalog: string
  summary: { key: string; value: string }[]
  lines: { id: string; label: string; detail: string; icon: string; tone: string; set: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightConversation {
  headline: string
  tone: string
  line: string
  total: string
  entries: number
  dominant: string
  workHeavy: boolean
  userLed: boolean
  repliesPerMessage: number
  stepsPerMessage: number
  summary: { key: string; value: string }[]
  kinds: { kind: string; label: string; count: number; icon: string; tone: string; percent: number; bar: string }[]
}

export interface InsightFailures {
  headline: string
  tone: string
  total: string
  refusals: number
  rate: number
  network: number
  summary: { key: string; value: string }[]
  categories: { kind: string; count: number; icon: string; tone: string; percent: number; bar: string }[]
  items: { name: string; kind: string; reason: string; age: string; icon: string; tone: string }[]
}

export interface InsightApprovals {
  headline: string
  tone: string
  total: string
  waiting: number
  urgent: number
  oldest: string
  bar: string
  summary: { key: string; value: string }[]
  items: { id: string; title: string; detail: string; age: string; icon: string; tone: string; urgent: boolean }[]
}

export interface InsightTimezone {
  headline: string
  tone: string
  local: string
  primary: string
  zones: number
  schedules: string
  multi: boolean
  remote: boolean
  summary: { key: string; value: string }[]
  all: { name: string; label: string; icon: string; tone: string; count: number; local: boolean; percent: number; bar: string }[]
  remoteList: { label: string; count: number }[]
}

export interface InsightDigest {
  headline: string
  oneLine: string
  label: string
  tone: string
  score: number
  areas: string
  onTrack: number
  attention: number
  summary: { key: string; value: string }[]
  highlights: { id: string; label: string; value: string; icon: string; tone: string }[]
  stats: { id: string; label: string; value: string; icon: string; tone: string }[]
}

export interface InsightAdvice {
  headline: string
  label: string
  tone: string
  score: number
  total: string
  urgent: number
  summary: { key: string; value: string }[]
  items: {
    id: string; title: string; detail: string; icon: string; tone: string;
    priority: number; priorityLabel: string; priorityIcon: string; priorityTone: string
  }[]
}

export interface InsightSession {
  headline: string
  label: string
  tone: string
  score: number
  session: string
  sessionIcon: string
  sessionTone: string
  revision: string
  unread: string
  bar: string
  deleted: boolean
  summary: { key: string; value: string }[]
  lines: { id: string; label: string; detail: string; icon: string; tone: string; ok: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightWorkspace {
  headline: string
  label: string
  tone: string
  score: number
  folder: string
  folderIcon: string
  folderTone: string
  set: number
  summary: { key: string; value: string }[]
  lines: { id: string; label: string; detail: string; icon: string; tone: string; set: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightMemory {
  headline: string
  label: string
  tone: string
  score: number
  shared: string
  sharedIcon: string
  sharedTone: string
  sharedDetail: string
  harness: boolean
  harnessIcon: string
  harnessTone: string
  skills: number
  on: number
  hasAny: boolean
  summary: { key: string; value: string }[]
  lines: { id: string; title: string; detail: string; icon: string; tone: string; on: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightTuning {
  headline: string
  tone: string
  count: string
  total: number
  on: number
  off: number
  fast: boolean
  effort: string
  effortIcon: string
  reasoning: boolean
  summary: { key: string; value: string }[]
  options: { key: string; title: string; value: string; icon: string; tone: string }[]
  enabled: { title: string; value: string; icon: string }[]
}

export interface InsightConnectors {
  headline: string
  tone: string
  total: number
  distinct: number
  duplicates: string[]
  chat: boolean
  mail: boolean
  code: boolean
  money: boolean
  summary: { key: string; value: string }[]
  categories: { kind: string; count: number; icon: string; tone: string; percent: number; bar: string }[]
  connectors: { name: string; category: string; icon: string; tone: string; detail: string }[]
}

export interface InsightIdentity {
  name: string
  headline: string
  tone: string
  band: string
  bandIcon: string
  bandTone: string
  section: string
  mode: string
  rank: number
  color: string
  colorIcon: string
  shape: string
  look: string
  visible: boolean
  score: number
  count: string
  summary: { key: string; value: string }[]
  traits: { id: string; label: string; detail: string; icon: string; tone: string; on: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightResilience {
  headline: string
  label: string
  tone: string
  score: number
  bar: string
  chain: string
  total: number
  hasChain: boolean
  hasOverride: boolean
  customCommands: number
  selfFallbacks: boolean
  switches: { label: string; on: boolean }[]
  summary: { key: string; value: string }[]
  fallbacks: { backend: string; model: string; command: string; label: string; icon: string; tone: string }[]
}

export interface InsightTrust {
  mode: string
  modeLabel: string
  modeIcon: string
  modeTone: string
  modeDetail: string
  score: number
  tone: string
  label: string
  headline: string
  total: number
  countLabel: string
  broad: number
  command: number
  hasDeny: boolean
  hasBroadAllow: boolean
  asksForEverything: boolean
  decisions: { decision: string; count: number }[]
  summary: { key: string; value: string }[]
  rules: { pattern: string; kind: string; decision: string; icon: string; tone: string; broad: boolean }[]
}

export interface InsightLibrary {
  headline: string
  tone: string
  count: string
  total: number
  distinct: number
  duplicates: string[]
  hasScript: boolean
  hasReference: boolean
  hasAsset: boolean
  summary: { key: string; value: string }[]
  kinds: { kind: string; count: number; icon: string; tone: string; percent: number; bar: string }[]
  skills: { name: string; kind: string; icon: string; tone: string; detail: string }[]
}

export interface InsightRoutineSummary {
  count: string
  active: number
  paused: number
  scheduled: number
  activePercent: number
  bar: string
  next: string
  mix: string
  clock: boolean
  event: boolean
  allScheduled: boolean
  headline: string
  tone: string
  summary: { key: string; value: string }[]
  soonest: { id: string; name: string; enabled: boolean; trigger: string; triggerIcon: string; nextIn: string; stateIcon: string; stateTone: string }[]
  pausedList: { id: string; name: string }[]
}

export interface InsightWeek {
  headline: string
  tone: string
  verdict: string
  total: number
  totalLabel: string
  average: number
  peak: { label: string; count: number } | null
  quietDays: number
  activeStreak: number
  quietStreak: number
  aboveAverage: number
  change: number
  perfectAttendance: boolean
  grew: boolean
  trend: { kind: string; label: string; icon: string; tone: string }
  top: { label: string; count: number; bar: string }[]
  quiet: { label: string; count: number; bar: string }[]
  days: { key: string; label: string; count: number }[]
}

export interface InsightProfile {
  headline: string
  enabled: number
  total: number
  missing: number
  coverage: number
  bar: string
  nextUp: { title: string; icon: string }[]
  caps: { id: string; title: string; detail: string; icon: string; tone: string; enabled: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightHealth {
  score: number
  label: string
  tone: string
  headline: string
  blocking: boolean
  errors: number
  warnings: number
  counts: { severity: string; count: number }[]
  checks: { id: string; title: string; detail: string; severity: string; tone: string; icon: string }[]
}

export interface InsightActivity {
  headline: string
  steps: number
  errors: number
  dominant: string
  span: string
  total: number
  counts: { kind: string; count: number; percent: number }[]
  groups: { kind: string; label: string; icon: string; tone: string; count: number }[]
  items: { text: string; detail: string; status: string; icon: string; tone: string }[]
  days: { day: string; label: string; count: number; width: number }[]
}

export interface InsightRoutine {
  id: string
  name: string
  enabled: boolean
  schedules: string[]
  nextIn: string | null
  nextInMs: number | null
}

export interface BotInsights {
  botId: string
  description: string
  model: string
  provider: string | null
  recentWork: string[]
  activity: InsightActivity
  health: InsightHealth
  profile: InsightProfile
  week: InsightWeek
  routineSummary: InsightRoutineSummary
  library: InsightLibrary
  trust: InsightTrust
  resilience: InsightResilience
  identity: InsightIdentity
  connectors: InsightConnectors
  tuning: InsightTuning
  memory: InsightMemory
  workspace: InsightWorkspace
  session: InsightSession
  advice: InsightAdvice
  digest: InsightDigest
  timezone: InsightTimezone
  approvals: InsightApprovals
  failures: InsightFailures
  conversation: InsightConversation
  agent: InsightAgentProfile
  fresh: InsightFresh
  prompts: InsightPrompts
  directory: InsightDirectory
  blocks: InsightBlocks
  access: InsightAccess
  dictation: InsightDictation
  liveness: InsightLiveness
  context: InsightContext
  opendots: InsightOpenDots
  openbot: InsightOpenBot
  agenteschat: InsightAgentesChat
  textfmt: InsightTextFmt
  localfmt: InsightLocalFmt
  markdown: InsightMarkdown
  notify: InsightNotify
  routines: InsightRoutine[]
  usage: {
    used: number
    usedText: string
    limit: number | null
    resetsAt: number | null
  }
  skills: {
    total: number
    resources: Record<string, number>
  }
}
