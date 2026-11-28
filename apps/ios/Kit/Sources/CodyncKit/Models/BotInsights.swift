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
    public let blueprint: InsightBlueprint
    public let review: InsightReview
    public let history: InsightHistory
    public let apierror: InsightApiError
    public let memoryctx: InsightMemoryCtx
    public let searchextra: InsightSearchExtra
    public let evalextra: InsightEvalExtra
    public let connextra: InsightConnExtra
    public let shortcuts: InsightShortcuts
    public let palette: InsightPalette
    public let picker: InsightPicker
    public let contextselect: InsightContextSelect
    public let mdtables: InsightMdTables
    public let contextpin: InsightContextPin
    public let verify: InsightVerify
    public let autofill: InsightAutofill
    public let rrule: InsightRrule
    public let iteration: InsightIteration
    public let approvalrules: InsightApprovalRules
    public let brainpicker: InsightBrainPicker
    public let engine: InsightEngine
    public let accounts: InsightAccounts
    public let plans: InsightPlans
    public let providers: InsightProviders
    public let avatar: InsightAvatar
    public let secretcleanup: InsightSecretCleanup
    public let toolreplay: InsightToolReplay
    public let vault: InsightVault
    public let devices: InsightDevices
    public let noai: InsightNoAi
    public let missions: InsightMissions
    public let cmdpolicy: InsightCmdPolicy
    public let router: InsightRouter
    public let modelprefs: InsightModelPrefs
    public let botstore: InsightBotStore
    public let promptactions: InsightPromptActions
    public let csiterm: InsightCsiTerm
    public let plugins: InsightPlugins
    public let service: InsightService
    public let draftstash: InsightDraftStash
    public let layout: InsightLayout
    public let channelaccess: InsightChannelAccess
    public let work: InsightWork
    public let contextblocks: InsightContextBlocks
    public let toollabels: InsightToolLabels
    public let routines: [InsightRoutine]
    public let usage: InsightUsage
    public let skills: InsightSkills
}

/// A host-computed eval metrics summary from the child-agent port.
public struct InsightEvalExtra: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let lines: Int
    public let code: Int
    public let median: Int
    public let p90: Int
    public let over1k: Int
    public let over2k: Int
    public let skip: Bool
    public let module: String
    public let verdicts: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed connection selection summary from the child-agent port.
public struct InsightConnExtra: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let channels: [String]
    public let providers: [String]
    public let selected: Int
    public let rows: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed tool labels summary.
public struct InsightToolLabels: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let key: String
    public let app: String?
    public let label: String?
    public let labelPt: String?
    public let humanized: String
    public let matched: Bool
    public let technical: Bool
    public let subject: String
    public let phrases: String
    public let line: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed context blocks summary.
public struct InsightContextBlocks: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let blocks: Int
    public let turns: Int
    public let found: Int
    public let noReply: Bool
    public let reaction: Bool
    public let folded: [String]
    public let conversation: String
    public let stable: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed work header summary.
public struct InsightWorkHeader: Decodable, Sendable {
    public let steps: String
    public let connectors: String?
    public let duration: String?
}

/// A host-computed work summary.
public struct InsightWork: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let steps: Int
    public let calls: Int
    public let folded: Int
    public let duration: String
    public let state: String
    public let apps: Int
    public let quiet: Bool
    public let header: InsightWorkHeader
    public let summary: [InsightSummaryRow]
}

/// A host-computed channel access summary.
public struct InsightChannelAccess: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let channels: Int
    public let summaryLine: String
    public let hasAdmins: Bool
    public let pairing: Bool
    public let approved: Int
    public let provider: String
    public let creator: String
    public let api: String
    public let complete: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed pane layout summary.
public struct InsightLayout: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let panes: Int
    public let groups: Int
    public let depth: Int
    public let orientation: String
    public let drop: String
    public let normalized: Bool
    public let active: String
    public let ids: [String]
    public let weights: [Double]
    public let summary: [InsightSummaryRow]
}

/// A host-computed composer draft stash summary.
public struct InsightDraftStash: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let key: String
    public let newKey: String
    public let stamp: Int
    public let entries: Int
    public let drafts: Int
    public let cleared: String
    public let merged: Bool
    public let title: String
    public let compacted: Int
    public let legacy: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed service launch summary.
public struct InsightService: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let taskName: String
    public let runKey: String
    public let unit: String
    public let modes: Int
    public let platforms: String
    public let parsed: [String]
    public let pathMac: String
    public let pathLinux: String
    public let drift: Bool
    public let quote: String
    public let running: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed plugin store summary.
public struct InsightPlugins: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let root: String
    public let staging: String
    public let settings: String
    public let validId: Bool
    public let reserved: Bool
    public let toolName: Bool
    public let relativePath: Bool
    public let manifestVersion: Int
    public let apiVersion: String
    public let compatible: Bool
    public let problem: String?
    public let staleLock: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed CSI terminal summary.
public struct InsightCsiTerm: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let prefix: String
    public let cursorHome: String
    public let eraseLine: String
    public let eraseScreen: String
    public let cursorStyle: String
    public let paramByte: Bool
    public let intermediateByte: Bool
    public let finalByte: Bool
    public let move: String
    public let eraseLines: String
    public let scroll: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed prompt action summary.
public struct InsightPromptActions: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let idCandidate: Bool
    public let busyRetryMs: Int
    public let retryIntervalMs: Int
    public let ttlMs: Int
    public let timeout: Bool
    public let notOwned: Bool
    public let surface: String?
    public let forwarded: Bool
    public let inFlight: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed bot store summary.
public struct InsightBotStore: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let id: String
    public let name: String
    public let folder: String
    public let deliverable: String
    public let valid: Bool
    public let errors: [String]
    public let schedules: Int
    public let fleet: Bool
    public let dangerous: Bool
    public let dangerousTools: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed model preferences summary.
public struct InsightModelPrefs: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let levels: [String]
    public let openaiLevels: [String]
    public let anthropicLevels: [String]
    public let modelDefault: String
    public let chat: String?
    public let adaptive: Bool
    public let label: String
    public let humanized: String
    public let openrouter: Bool
    public let prefs: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed router threshold.
public struct InsightRouterThreshold: Decodable, Sendable {
    public let name: String
    public let value: Double
}

/// A host-computed router summary.
public struct InsightRouter: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let thresholds: [InsightRouterThreshold]
    public let quietMs: Int
    public let addressee: Bool
    public let selected: [String]
    public let everyone: Bool
    public let answered: Bool
    public let scores: [String]
    public let social: String?
    public let strategy: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed command policy summary.
public struct InsightCmdPolicy: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let words: Int
    public let commands: Int
    public let verdict: String
    public let denied: Bool
    public let secretPath: Bool
    public let readOnly: Bool
    public let repo: String?
    public let pushes: Int
    public let inline: Bool
    public let denyKinds: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed mission counts summary.
public struct InsightMissionCounts: Decodable, Sendable {
    public let total: Int
    public let done: Int
    public let doing: Int
    public let waiting: Int
}

/// A host-computed mission waiting part.
public struct InsightMissionWaiting: Decodable, Sendable {
    public let key: String
    public let count: Int
}

/// A host-computed mission composer command.
public struct InsightMissionCommand: Decodable, Sendable {
    public let command: String
    public let arg: String
}

/// A host-computed mission pull request.
public struct InsightMissionPr: Decodable, Sendable {
    public let number: String
    public let url: String?
}

/// A host-computed missions summary.
public struct InsightMissions: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let cells: [String]
    public let counts: InsightMissionCounts
    public let open: Bool
    public let waiting: [InsightMissionWaiting]
    public let tick: Int
    public let command: InsightMissionCommand?
    public let pr: InsightMissionPr?
    public let span: String
    public let commands: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed no-AI card.
public struct InsightNoAiCard: Decodable, Sendable {
    public let agentId: String
    public let forUserId: String?
    public let intent: String
    public let provider: String?
    public let autoOpen: Bool?
    public let own: String?
    public let signedOut: Bool?
    public let planLine: String?
    public let model: String?
    public let why: String?
    public let missing: String?
    public let held: Bool?
}

/// A host-computed no-AI summary.
public struct InsightNoAi: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let card: InsightNoAiCard
    public let line: String
    public let state: String
    public let passes: Bool
    public let opens: Bool
    public let brain: String
    public let kind: String
    public let choices: Int
    public let firstChoice: String
    public let autoOpen: Bool?
    public let summary: [InsightSummaryRow]
}

/// A host-computed device portrait.
public struct InsightPortrait: Decodable, Sendable {
    public let kind: String
    public let state: String
}

/// A host-computed device stub in a connect stage.
public struct InsightDeviceStub: Decodable, Sendable {
    public let id: String?
    public let name: String?
}

/// A host-computed Android connect stage.
public struct InsightDeviceStage: Decodable, Sendable {
    public let kind: String
    public let phone: InsightDeviceStub?
}

/// A host-computed Android cable guide.
public struct InsightDeviceSteps: Decodable, Sendable {
    public let states: [String: String]
    public let now: String
    public let open: String
    public let byHand: [String]
}

/// A host-computed devices summary.
public struct InsightDevices: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let devices: Int
    public let phones: Int
    public let all: Int
    public let look: String
    public let portrait: InsightPortrait
    public let line: String
    public let rows: Int
    public let link: String
    public let stage: InsightDeviceStage
    public let steps: InsightDeviceSteps
    public let summary: [InsightSummaryRow]
}

/// A host-computed card summary inside the vault.
public struct InsightCardSummary: Decodable, Sendable {
    public let brand: String
    public let last4: String
    public let expMonth: Int
    public let expYear: Int
    public let name: String?
}

/// A host-computed parsed card expiry.
public struct InsightCardExpiry: Decodable, Sendable {
    public let expMonth: Int
    public let expYear: Int
}

/// A host-computed bot vault summary.
public struct InsightVault: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let authTypes: Int
    public let siteless: Bool
    public let variable: String
    public let problem: String?
    public let code: String?
    public let identifier: Bool
    public let otp: Bool
    public let card: InsightCardSummary
    public let title: String
    public let expiry: InsightCardExpiry?
    public let policy: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed tool replay summary.
public struct InsightToolReplay: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let calls: Int
    public let kept: Int
    public let budget: Int
    public let left: Int
    public let replay: Bool
    public let followUp: Bool
    public let credentialField: Bool
    public let secretTool: Bool
    public let outcome: String
    public let sample: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed secret cleanup summary.
public struct InsightSecretCleanup: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let reference: String
    public let kind: String
    public let prefix: String?
    public let known: Bool
    public let attemptMs: Int
    public let backupMs: Int
    public let storeMs: Int
    public let providerMs: Int
    public let retryDue: Int
    public let backupDue: Int
    public let expired: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed avatar color summary.
public struct InsightAvatar: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let color: String
    public let hex: String
    public let rgb: String
    public let eye: String
    public let ink: String
    public let bucket: String
    public let contrast: String
    public let invites: Int
    public let initials: String
    public let background: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed model provider summary.
public struct InsightProviders: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let providers: Int
    public let keys: Int
    public let models: Int
    public let usable: Int
    public let guess: String
    public let label: String
    public let site: String
    public let hasPlans: Bool
    public let billing: Bool
    public let privacy: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed coding plan summary.
public struct InsightPlans: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let plan: String
    public let providerPlan: String?
    public let models: Int
    public let choice: String
    public let freeHint: Bool
    public let keyPlan: Bool
    public let placeholder: String
    public let geminiPersonal: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed AI accounts summary.
public struct InsightAccounts: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let accounts: Int
    public let plans: Int
    public let state: String
    public let offersAgain: Bool
    public let title: String
    public let action: String
    public let actionIcon: String
    public let lendable: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed brain engine summary.
public struct InsightEngine: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let engine: String
    public let state: String
    public let off: Bool
    public let executor: String
    public let line: String
    public let modelBrain: Bool
    public let auto: Bool
    public let computers: [String]
    public let summary: [InsightSummaryRow]
}

/// A host-computed brain picker summary.
public struct InsightBrainPicker: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let description: String
    public let groups: [String]
    public let free: Int
    public let keywords: String
    public let auto: Bool
    public let connect: Bool
    public let label: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed approval rules summary.
public struct InsightApprovalRules: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let tool: String
    public let sqlRead: Bool
    public let target: String
    public let command: Bool
    public let maxActions: Int
    public let actions: Int
    public let rules: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed iteration bookkeeping summary.
public struct InsightIteration: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let used: Int
    public let max: Int
    public let warning: Bool
    public let warningText: String
    public let ghosts: Int
    public let tools: Int
    public let toolTail: Bool
    public let steerAfter: Int?
    public let restartLabel: String
    public let restartIcon: String
    public let refunded: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed recurrence rule summary.
public struct InsightRrule: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let freq: String
    public let interval: Int
    public let times: [String]
    public let weekdays: [String]
    public let cron: String?
    public let cronCompatible: Bool
    public let workdays: Bool
    public let error: String
    public let timezone: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed autofill classification summary.
public struct InsightAutofill: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let controls: Int
    public let otp: Int
    public let checkout: Int
    public let best: String
    public let tokens: Int
    public let paymentTokens: Int
    public let addressTokens: Int
    public let password: Bool
    public let summary: [InsightSummaryRow]
}

/// A host-computed verification-on-stop summary.
public struct InsightVerify: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let paths: Int
    public let dirs: Int
    public let proseOnly: Bool
    public let enabled: Bool
    public let evidence: String
    public let nudge: Bool
    public let nudgeLabel: String
    public let maxPaths: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed context window pin summary.
public struct InsightContextPin: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let window: Int
    public let marker: String
    public let suffix: String
    public let pinned: Bool
    public let fallback: Bool
    public let pinnedSuffix: String
    public let advertised: Int?
    public let disagrees: Bool
    public let known: Bool
    public let ratio: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed markdown table summary.
public struct InsightMdTables: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let rows: Int
    public let cols: Int
    public let wide: Int
    public let narrow: Bool
    public let layout: String
    public let divider: String
    public let aligned: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed context budget summary.
public struct InsightContextSelect: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let estimate: Int
    public let available: Int
    public let fixed: Int
    public let window: Int
    public let output: Int
    public let fits: Bool
    public let tooSmall: Bool
    public let strategy: String
    public let cache: String
    public let cacheIcon: String
    public let kept: Int
    public let omitted: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed model-picker usage summary.
public struct InsightPicker: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let bar: String
    public let used: String
    public let left: String
    public let style: String
    public let resets: String
    public let featured: [String]
    public let gauges: Int
    public let levels: Int
    public let width: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed command palette summary.
public struct InsightPalette: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let command: String
    public let needle: String
    public let results: Int
    public let recentPages: Int
    public let commands: Int
    public let bots: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed keyboard shortcut summary.
public struct InsightShortcuts: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let modifier: String
    public let quote: String
    public let commandMenu: String?
    public let primary: Bool
    public let browserShift: Bool
    public let resolved: [InsightShortcutRow]
    public let summary: [InsightSummaryRow]
}

public struct InsightShortcutRow: Decodable, Sendable {
    public let command: String
    public let label: String
    public let icon: String
    public let tone: String
}

/// A host-computed ranked-search summary from the child-agent port.
public struct InsightSearchExtra: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let query: String
    public let terms: [String]
    public let total: Int
    public let returned: Int
    public let hasMore: Bool
    public let empty: Bool
    public let score: String
    public let totalLabel: String
    public let ops: Int
    public let sort: String
    public let matched: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed memory-context sanitisation summary.
public struct InsightMemoryCtx: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let fenced: Int
    public let notes: Int
    public let hidden: Int
    public let preview: String
    public let block: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed provider API error summary.
public struct InsightApiError: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let icon: String
    public let `class`: String
    public let classLabel: String
    public let status: String
    public let retryable: Bool
    public let streamParse: Bool
    public let network: Bool
    public let samples: Int
    public let preview: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed history commentary summary.
public struct InsightHistory: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let messages: Int
    public let commentary: Int
    public let finals: Int
    public let withheld: Int
    public let preview: String
    public let visible: Int
    public let summary: [InsightSummaryRow]
}

/// A host-computed idle review queue summary.
public struct InsightReview: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let mode: String
    public let modeIcon: String
    public let maxAge: Double
    public let settled: Bool
    public let live: Int
    public let queued: Int
    public let dispatchable: [String]
    public let pollMs: Int
    public let idle: String
    public let summary: [InsightSummaryRow]
}

/// A host-computed automation blueprint summary.
public struct InsightBlueprint: Decodable, Sendable {
    public let headline: String
    public let tone: String
    public let template: String
    public let cron: String
    public let missing: [String]
    public let slots: Int
    public let schedule: String
    public let slash: String
    public let deeplink: String
    public let time: String?
    public let valid: Bool
    public let summary: [InsightSummaryRow]
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
