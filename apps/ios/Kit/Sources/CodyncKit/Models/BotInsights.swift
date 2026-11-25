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
    public let routines: [InsightRoutine]
    public let usage: InsightUsage
    public let skills: InsightSkills
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
