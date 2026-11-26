import Foundation

/// A read-only summary of one bot, computed by the host from its own data and the bdots helpers.
public struct BotInsights: Decodable, Sendable {
    public let botId: String
    public let description: String
    public let model: String
    public let provider: String?
    public let recentWork: [String]
    public let activity: InsightActivity
    public let health: InsightHealth
    public let profile: InsightProfile
    public let week: InsightWeek
    public let routineSummary: InsightRoutineSummary
    public let library: InsightLibrary
    public let trust: InsightTrust
    public let resilience: InsightResilience
    public let identity: InsightIdentity
    public let connectors: InsightConnectors
    public let tuning: InsightTuning
    public let memory: InsightMemory
    public let workspace: InsightWorkspace
    public let session: InsightSession
    public let advice: InsightAdvice
    public let digest: InsightDigest
    public let timezone: InsightTimezone
    public let approvals: InsightApprovals
    public let failures: InsightFailures
    public let conversation: InsightConversation
    public let agent: InsightAgentProfile
    public let fresh: InsightFresh
    public let prompts: InsightPrompts
    public let directory: InsightDirectory
    public let blocks: InsightBlocks
    public let access: InsightAccess
    public let dictation: InsightDictation
    public let liveness: InsightLiveness
    public let context: InsightContext
    public let opendots: InsightOpenDots
    public let openbot: InsightOpenBot
    public let agenteschat: InsightAgentesChat
    public let textfmt: InsightTextFmt
    public let localfmt: InsightLocalFmt
    public let markdown: InsightMarkdown
    public let notify: InsightNotify
    public let mailtext: InsightMailText
    public let connstate: InsightConnState
    public let eval: InsightEval
    public let search: InsightSearch
    public let appearance: InsightAppearance
    public let quota: InsightQuota
    public let i18n: InsightI18n
    public let stream: InsightStream
    public let routines: [InsightRoutine]
    public let usage: InsightUsage
    public let skills: InsightSkills
}

/// A host-computed stream delivery summary.
public struct InsightStream: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let held: Int
    public let consumers: Int
    public let chunks: Int
    public let remaining: String
    public let writer: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed locale and plural summary.
public struct InsightI18n: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let code: String
    public let native: String
    public let english: String
    public let intl: String
    public let week: Int
    public let weekLabel: String
    public let script: String
    public let direction: String
    public let rtl: Bool
    public let plural: String
    public let pluralFrom: String
    public let categories: [String]
    public let locales: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed account quota summary.
public struct InsightQuota: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let usedPct: Int
    public let remaining: Int
    public let reset: String
    public let resetPhrase: String
    public let depleted: Bool
    public let low: Bool
    public let scope: String
    public let scopeIcon: String
    public let status: String
    public let windows: Int
    public let bpFull: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed appearance and colour token summary.
public struct InsightAppearance: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let preference: String
    public let appearance: String
    public let icon: String
    public let background: String
    public let text: String
    public let foreground: String
    public let accent: String
    public let radius: String
    public let bodySize: Int
    public let bodyLine: Int
    public let fontScale: Double
    public let maxScale: Double
    public let botColor: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed connection state summary.
public struct InsightConnState: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let selected: String
    public let state: String
    public let connected: Int
    public let channels: Int
    public let stale: Int
    public let backoff: Int
    public let icon: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed evaluation summary.
public struct InsightEval: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let score: Int
    public let verdict: String
    public let threshold: Int
    public let samples: Int
    public let grade: String
    public let icon: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed search summary.
public struct InsightSearch: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let query: String
    public let hits: Int
    public let filtered: Int
    public let score: Int
    public let empty: Bool
    public let icon: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed mailbox and database text summary.
public struct InsightMailText: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let address: String
    public let mailbox: String
    public let domain: String
    public let valid: Bool
    public let keys: Int
    public let addresses: Int
    public let rows: Int
    public let columns: Int
    public let cut: Bool
    public let maxBody: Int
    public let maxRows: Int
    public let maxValue: Int
    public let preview: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed notification and delivery summary.
public struct InsightNotify: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let state: String
    public let stateIcon: String
    public let marker: String
    public let diagnostic: Int
    public let freeform: Int
    public let delivered: Int
    public let failed: Int
    public let muted: Bool
    public let maxOutput: Int
    public let maxAttempts: Int
    public let floodDefault: Double
    public let floodCap: Double
    public let retryDelay: Double
    public let obligation: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed Markdown prose summary.
public struct InsightMarkdown: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let preview: String
    public let blocks: Int
    public let links: Int
    public let mentions: Int
    public let wide: Bool
    public let heading: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed locale clock and number summary.
public struct InsightLocalFmt: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let locale: String
    public let clock: String
    public let day: String
    public let weekday: String
    public let shortDate: String
    public let month: String
    public let relative: String
    public let timeAgo: String
    public let count: String
    public let tokens: String
    public let spend: String
    public let budget: String
    public let ordinal: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed text hygiene summary.
public struct InsightTextFmt: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let slug: String
    public let handle: String
    public let excerpt: String
    public let validEmails: Int
    public let invalidEmails: Int
    public let hidden: Int
    public let passwordLike: Int
    public let typoFix: String
    public let summary: [InsightSummaryRow]
}

/// Agentes-chat-style context budget summary.
public struct InsightAgentesChat: Decodable, Sendable {
    public let headline: String
    public let window: String
    public let windowIcon: String
    public let windowTone: String
    public let used: String
    public let percent: Int
    public let auto: String
    public let autoTone: String
    public let autoEfforts: [String]
    public let timeout: Int
    public let placeKept: Int
    public let groupWindow: Int
    public let summary: [InsightSummaryRow]
}

/// OpenBot-style run model summary.
public struct InsightOpenBot: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let engine: String
    public let model: String
    public let effort: String
    public let effortIcon: String
    public let effortTone: String
    public let route: String
    public let routeIcon: String
    public let plan: Bool
    public let thread: String
    public let responsesApi: Bool
    public let summary: [InsightSummaryRow]
}

/// OpenDots-style Learning and internal message summary.
public struct InsightOpenDots: Decodable, Sendable {
    public let headline: String
    public let container: String
    public let valid: Bool
    public let delivery: Bool
    public let deliveryError: String
    public let learningIcon: String
    public let learningTone: String
    public let internal: Int
    public let scheduledLabel: String
    public let voiceLabel: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed context pruning summary.
public struct InsightContext: Decodable, Sendable {
    public let headline: String
    public let page: String
    public let screenshots: String
    public let pageKeep: Int
    public let screenshotKeep: Int
    public let staleMin: Int
    public let pageNote: String
    public let screenshotNote: String
    public let pruned: String
    public let tone: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed liveness and delivery summary.
public struct InsightLiveness: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let state: String
    public let icon: String
    public let active: Bool
    public let age: String
    public let generation: Int
    public let heartbeat: Int
    public let due: Bool
    public let outcome: String
    public let outcomeIcon: String
    public let outcomeTone: String
    public let summary: [InsightSummaryRow]
}

/// Voice dictation settings from host configuration.
public struct InsightDictation: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let engine: String
    public let engineIcon: String
    public let engineTone: String
    public let model: String
    public let language: String
    public let supported: Bool
    public let needsCommit: Bool
    public let commitMs: Int
    public let count: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed public link access summary.
public struct InsightAccess: Decodable, Sendable {
    public let mode: String
    public let modeIcon: String
    public let modeTone: String
    public let canWrite: Bool
    public let form: Bool
    public let count: String
    public let collections: [InsightAccessCollection]
    public let invalid: [String]
    public let routing: String
    public let routingIcon: String
    public let routingTone: String
    public let liveName: String
    public let summary: [InsightSummaryRow]
}

public struct InsightAccessCollection: Decodable, Sendable {
    public let name: String
    public let valid: Bool
}

/// A host-computed transcript view of the last message.
public struct InsightBlocks: Decodable, Sendable {
    public let headline: String
    public let count: String
    public let said: Int
    public let work: Int
    public let workOnly: Bool
    public let quiet: Bool
    public let origin: String
    public let originIcon: String
    public let summary: [InsightSummaryRow]
    public let blocks: [InsightMessageBlock]
}

public struct InsightMessageBlock: Decodable, Sendable {
    public let kind: String
    public let text: String
    public let icon: String
    public let tone: String
    public let says: Bool
}

/// A host-computed directory summary for one bot.
public struct InsightDirectory: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let total: String
    public let ready: Int
    public let unread: Int
    public let busy: Int
    public let percent: Int
    public let bar: String
    public let directory: String
    public let email: String
    public let routable: Bool
    public let routeIcon: String
    public let routeTone: String
    public let reason: String
    public let summary: [InsightSummaryRow]
    public let integrations: [InsightDirectoryIntegration]
}

public struct InsightDirectoryIntegration: Decodable, Sendable {
    public let id: String
    public let name: String
    public let endpoint: Bool
    public let header: String
}

/// A host-computed prompt style summary for one bot.
public struct InsightPrompts: Decodable, Sendable {
    public let model: String
    public let context: String
    public let price: String
    public let valid: Bool
    public let checkIn: String
    public let zone: String
    public let clock: String
    public let watch: String
    public let length: String
    public let instruction: String
    public let summary: [InsightSummaryRow]
    public let lines: [String]
}

/// Helpers ported from the latest bdots source, shaped for display.
public struct InsightFresh: Decodable, Sendable {
    public let agent: String
    public let deployment: String
    public let language: String
    public let receipt: String
    public let profile: String
    public let summary: [InsightSummaryRow]
    public let titles: InsightFreshTitles
    public let resources: [InsightFreshResource]
}

public struct InsightFreshTitles: Decodable, Sendable {
    public let exec: String
    public let write: String
}

public struct InsightFreshResource: Decodable, Sendable {
    public let path: String
    public let kind: String
    public let script: Bool
    public let reference: Bool
    public let asset: Bool
}

/// A host-computed agent profile for one bot.
public struct InsightAgentProfile: Decodable, Sendable {
    public let headline: String
    public let label: String
    public let tone: String
    public let score: Int
    public let account: String
    public let accountIcon: String
    public let accountTone: String
    public let mode: String
    public let modeIcon: String
    public let modeTone: String
    public let fast: Bool
    public let fastIcon: String
    public let fastTone: String
    public let catalog: String
    public let summary: [InsightSummaryRow]
    public let lines: [InsightAgentLine]
}

public struct InsightAgentLine: Decodable, Sendable {
    public let id: String
    public let label: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let set: Bool
    public let stateIcon: String
    public let stateTone: String
}

/// A host-computed summary of a bot's chat rhythm.
public struct InsightConversation: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let line: String
    public let total: String
    public let entries: Int
    public let dominant: String
    public let workHeavy: Bool
    public let userLed: Bool
    public let repliesPerMessage: Int
    public let stepsPerMessage: Int
    public let summary: [InsightSummaryRow]
    public let kinds: [InsightConversationKind]
}

public struct InsightConversationKind: Decodable, Sendable {
    public let kind: String
    public let label: String
    public let count: Int
    public let icon: String
    public let tone: String
    public let percent: Int
    public let bar: String
}

/// A host-computed summary of recent tool failures.
public struct InsightFailures: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let total: String
    public let refusals: Int
    public let rate: Int
    public let network: Int
    public let summary: [InsightSummaryRow]
    public let categories: [InsightFailureCategory]
    public let items: [InsightFailureItem]
}

public struct InsightFailureCategory: Decodable, Sendable {
    public let kind: String
    public let count: Int
    public let icon: String
    public let tone: String
    public let percent: Int
    public let bar: String
}

public struct InsightFailureItem: Decodable, Sendable {
    public let name: String
    public let kind: String
    public let reason: String
    public let age: String
    public let icon: String
    public let tone: String
}

/// A host-computed summary of approval cards waiting on the user.
public struct InsightApprovals: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let total: String
    public let waiting: Int
    public let urgent: Int
    public let oldest: String
    public let bar: String
    public let summary: [InsightSummaryRow]
    public let items: [InsightApprovalItem]
}

public struct InsightApprovalItem: Decodable, Sendable {
    public let id: String
    public let title: String
    public let detail: String
    public let age: String
    public let icon: String
    public let tone: String
    public let urgent: Bool
}

/// A host-computed summary of the time zones a bot's routines use.
public struct InsightTimezone: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let local: String
    public let primary: String
    public let zones: Int
    public let schedules: String
    public let multi: Bool
    public let remote: Bool
    public let summary: [InsightSummaryRow]
    public let all: [InsightZoneLine]
    public let remoteList: [InsightRemoteZone]
}

public struct InsightZoneLine: Decodable, Sendable {
    public let name: String
    public let label: String
    public let icon: String
    public let tone: String
    public let count: Int
    public let local: Bool
    public let percent: Int
    public let bar: String
}

public struct InsightRemoteZone: Decodable, Sendable {
    public let label: String
    public let count: Int
}

/// A host-computed executive digest for one bot.
public struct InsightDigest: Decodable, Sendable {
    public let headline: String
    public let oneLine: String
    public let label: String
    public let tone: String
    public let score: Int
    public let areas: String
    public let onTrack: Int
    public let attention: Int
    public let summary: [InsightSummaryRow]
    public let highlights: [InsightDigestStat]
    public let stats: [InsightDigestStat]
}

public struct InsightDigestStat: Decodable, Sendable {
    public let id: String
    public let label: String
    public let value: String
    public let icon: String
    public let tone: String
}

/// A host-computed, prioritised suggestion list for one bot.
public struct InsightAdvice: Decodable, Sendable {
    public let headline: String
    public let label: String
    public let tone: String
    public let score: Int
    public let total: String
    public let urgent: Int
    public let summary: [InsightSummaryRow]
    public let items: [InsightAdviceItem]
}

public struct InsightAdviceItem: Decodable, Sendable {
    public let id: String
    public let title: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let priority: Int
    public let priorityLabel: String
    public let priorityIcon: String
    public let priorityTone: String
}

/// A host-computed session and sync state for one bot.
public struct InsightSession: Decodable, Sendable {
    public let headline: String
    public let label: String
    public let tone: String
    public let score: Int
    public let session: String
    public let sessionIcon: String
    public let sessionTone: String
    public let revision: String
    public let unread: String
    public let bar: String
    public let deleted: Bool
    public let summary: [InsightSummaryRow]
    public let lines: [InsightSessionLine]
}

public struct InsightSessionLine: Decodable, Sendable {
    public let id: String
    public let label: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let ok: Bool
    public let stateIcon: String
    public let stateTone: String
}

/// A host-computed summary of one bot's work environment.
public struct InsightWorkspace: Decodable, Sendable {
    public let headline: String
    public let label: String
    public let tone: String
    public let score: Int
    public let folder: String
    public let folderIcon: String
    public let folderTone: String
    public let set: Int
    public let summary: [InsightSummaryRow]
    public let lines: [InsightWorkspaceLine]
}

public struct InsightWorkspaceLine: Decodable, Sendable {
    public let id: String
    public let label: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let set: Bool
    public let stateIcon: String
    public let stateTone: String
}

/// A host-computed memory posture for one bot.
public struct InsightMemory: Decodable, Sendable {
    public let headline: String
    public let label: String
    public let tone: String
    public let score: Int
    public let shared: String
    public let sharedIcon: String
    public let sharedTone: String
    public let sharedDetail: String
    public let harness: Bool
    public let harnessIcon: String
    public let harnessTone: String
    public let skills: Int
    public let on: Int
    public let hasAny: Bool
    public let summary: [InsightSummaryRow]
    public let lines: [InsightMemoryLine]
}

public struct InsightMemoryLine: Decodable, Sendable {
    public let id: String
    public let title: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let on: Bool
    public let stateIcon: String
    public let stateTone: String
}

/// A host-computed summary of one bot's model tuning.
public struct InsightTuning: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let count: String
    public let total: Int
    public let on: Int
    public let off: Int
    public let fast: Bool
    public let effort: String
    public let effortIcon: String
    public let reasoning: Bool
    public let summary: [InsightSummaryRow]
    public let options: [InsightTuningOption]
    public let enabled: [InsightTuningEnabled]
}

public struct InsightTuningOption: Decodable, Sendable {
    public let key: String
    public let title: String
    public let value: String
    public let icon: String
    public let tone: String
}

public struct InsightTuningEnabled: Decodable, Sendable {
    public let title: String
    public let value: String
    public let icon: String
}

/// A host-computed summary of one bot's connected accounts.
public struct InsightConnectors: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let total: Int
    public let distinct: Int
    public let duplicates: [String]
    public let chat: Bool
    public let mail: Bool
    public let code: Bool
    public let money: Bool
    public let summary: [InsightSummaryRow]
    public let categories: [InsightConnectorCategory]
    public let connectors: [InsightConnectorLine]
}

public struct InsightConnectorCategory: Decodable, Sendable {
    public let kind: String
    public let count: Int
    public let icon: String
    public let tone: String
    public let percent: Int
    public let bar: String
}

public struct InsightConnectorLine: Decodable, Sendable {
    public let name: String
    public let category: String
    public let icon: String
    public let tone: String
    public let detail: String
}

/// A host-computed roster identity for one bot.
public struct InsightIdentity: Decodable, Sendable {
    public let name: String
    public let headline: String
    public let tone: String
    public let band: String
    public let bandIcon: String
    public let bandTone: String
    public let section: String
    public let mode: String
    public let rank: Int
    public let color: String
    public let colorIcon: String
    public let shape: String
    public let look: String
    public let visible: Bool
    public let score: Int
    public let count: String
    public let summary: [InsightSummaryRow]
    public let traits: [InsightIdentityTrait]
}

public struct InsightIdentityTrait: Decodable, Sendable {
    public let id: String
    public let label: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let on: Bool
    public let stateIcon: String
    public let stateTone: String
}

/// A host-computed resilience summary for one bot.
public struct InsightResilience: Decodable, Sendable {
    public let headline: String
    public let label: String
    public let tone: String
    public let score: Int
    public let bar: String
    public let chain: String
    public let total: Int
    public let hasChain: Bool
    public let hasOverride: Bool
    public let customCommands: Int
    public let selfFallbacks: Bool
    public let switches: [InsightSwitch]
    public let summary: [InsightSummaryRow]
    public let fallbacks: [InsightFallback]
}

public struct InsightSwitch: Decodable, Sendable {
    public let label: String
    public let on: Bool
}

public struct InsightFallback: Decodable, Sendable {
    public let backend: String
    public let model: String
    public let command: String
    public let label: String
    public let icon: String
    public let tone: String
}

/// A host-computed trust posture for one bot.
public struct InsightTrust: Decodable, Sendable {
    public let mode: String
    public let modeLabel: String
    public let modeIcon: String
    public let modeTone: String
    public let modeDetail: String
    public let score: Int
    public let tone: String
    public let label: String
    public let headline: String
    public let total: Int
    public let countLabel: String
    public let broad: Int
    public let command: Int
    public let hasDeny: Bool
    public let hasBroadAllow: Bool
    public let asksForEverything: Bool
    public let decisions: [InsightDecisionCount]
    public let summary: [InsightSummaryRow]
    public let rules: [InsightTrustRule]
}

public struct InsightDecisionCount: Decodable, Sendable {
    public let decision: String
    public let count: Int
}

public struct InsightTrustRule: Decodable, Sendable {
    public let pattern: String
    public let kind: String
    public let decision: String
    public let icon: String
    public let tone: String
    public let broad: Bool
}

/// A host-computed summary of one bot's skill library.
public struct InsightLibrary: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let count: String
    public let total: Int
    public let distinct: Int
    public let duplicates: [String]
    public let hasScript: Bool
    public let hasReference: Bool
    public let hasAsset: Bool
    public let summary: [InsightSummaryRow]
    public let kinds: [InsightLibraryKind]
    public let skills: [InsightLibrarySkill]
}

public struct InsightLibraryKind: Decodable, Sendable {
    public let kind: String
    public let count: Int
    public let icon: String
    public let tone: String
    public let percent: Int
    public let bar: String
}

public struct InsightLibrarySkill: Decodable, Sendable {
    public let name: String
    public let kind: String
    public let icon: String
    public let tone: String
    public let detail: String
}

/// A host-computed summary of one bot's routines.
public struct InsightRoutineSummary: Decodable, Sendable {
    public let count: String
    public let active: Int
    public let paused: Int
    public let scheduled: Int
    public let activePercent: Int
    public let bar: String
    public let next: String
    public let mix: String
    public let clock: Bool
    public let event: Bool
    public let allScheduled: Bool
    public let headline: String
    public let tone: String
    public let summary: [InsightSummaryRow]
    public let soonest: [InsightSoonestRoutine]
    public let pausedList: [InsightPausedRoutine]
}

public struct InsightSummaryRow: Decodable, Sendable {
    public let key: String
    public let value: String
}

public struct InsightSoonestRoutine: Decodable, Sendable {
    public let id: String
    public let name: String
    public let enabled: Bool
    public let trigger: String
    public let triggerIcon: String
    public let nextIn: String
    public let stateIcon: String
    public let stateTone: String
}

public struct InsightPausedRoutine: Decodable, Sendable {
    public let id: String
    public let name: String
}

/// A host-computed weekly summary for one bot.
public struct InsightWeek: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let verdict: String
    public let total: Int
    public let totalLabel: String
    public let average: Int
    public let peak: InsightPeak?
    public let quietDays: Int
    public let activeStreak: Int
    public let quietStreak: Int
    public let aboveAverage: Int
    public let change: Int
    public let perfectAttendance: Bool
    public let grew: Bool
    public let trend: InsightTrend
    public let top: [InsightDayBar]
    public let quiet: [InsightDayBar]
    public let days: [InsightDayCount]
}

public struct InsightPeak: Decodable, Sendable {
    public let label: String
    public let count: Int
}

public struct InsightTrend: Decodable, Sendable {
    public let kind: String
    public let label: String
    public let icon: String
    public let tone: String
}

public struct InsightDayBar: Decodable, Sendable {
    public let label: String
    public let count: Int
    public let bar: String
}

public struct InsightDayCount: Decodable, Sendable {
    public let key: String
    public let label: String
    public let count: Int
}

/// A host-computed capability profile for one bot.
public struct InsightProfile: Decodable, Sendable {
    public let headline: String
    public let enabled: Int
    public let total: Int
    public let missing: Int
    public let coverage: Int
    public let bar: String
    public let nextUp: [InsightNextUp]
    public let caps: [InsightCapability]
}

public struct InsightNextUp: Decodable, Sendable {
    public let title: String
    public let icon: String
}

public struct InsightCapability: Decodable, Identifiable, Sendable {
    public let id: String
    public let title: String
    public let detail: String
    public let icon: String
    public let tone: String
    public let enabled: Bool
    public let stateIcon: String
    public let stateTone: String
}

/// A host-computed configuration health report for one bot.
public struct InsightHealth: Decodable, Sendable {
    public let score: Int
    public let label: String
    public let tone: String
    public let headline: String
    public let blocking: Bool
    public let errors: Int
    public let warnings: Int
    public let counts: [InsightSeverityCount]
    public let checks: [InsightCheck]
}

public struct InsightSeverityCount: Decodable, Sendable {
    public let severity: String
    public let count: Int
}

public struct InsightCheck: Decodable, Identifiable, Sendable {
    public let id: String
    public let title: String
    public let detail: String
    public let severity: String
    public let tone: String
    public let icon: String
}

/// A host-computed activity timeline: steps folded into runs, a daily strip and a summary.
public struct InsightActivity: Decodable, Sendable {
    public let headline: String
    public let steps: Int
    public let errors: Int
    public let dominant: String
    public let span: String
    public let total: Int
    public let counts: [InsightCount]
    public let groups: [InsightGroup]
    public let items: [InsightStep]
    public let days: [InsightDay]
}

public struct InsightCount: Decodable, Sendable {
    public let kind: String
    public let count: Int
    public let percent: Int
}

public struct InsightGroup: Decodable, Identifiable, Sendable {
    public let kind: String
    public let label: String
    public let icon: String
    public let tone: String
    public let count: Int
    public var id: String { kind }
}

public struct InsightStep: Decodable, Sendable {
    public let text: String
    public let detail: String
    public let status: String
    public let icon: String
    public let tone: String
}

public struct InsightDay: Decodable, Identifiable, Sendable {
    public let day: String
    public let label: String
    public let count: Int
    public let width: Int
    public var id: String { day }
}

public struct InsightRoutine: Decodable, Identifiable, Sendable {
    public let id: String
    public let name: String
    public let enabled: Bool
    public let schedules: [String]
    public let nextIn: String?
    public let nextInMs: Int64?
}

public struct InsightUsage: Decodable, Sendable {
    public let used: Int64
    public let usedText: String
    public let limit: Int64?
    public let resetsAt: Int64?
}

public struct InsightSkills: Decodable, Sendable {
    public let total: Int
    public let resources: [String: Int]
}
