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
    public let routines: [InsightRoutine]
    public let usage: InsightUsage
    public let skills: InsightSkills
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
