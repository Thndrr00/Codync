import Foundation

/// A read-only summary of one bot, computed by the host from its own data and the bdots helpers.
public struct BotInsights: Decodable, Sendable {
    public let botId: String
    public let description: String
    public let model: String
    public let provider: String?
    public let recentWork: [String]
    public let activity: InsightActivity
    public let routines: [InsightRoutine]
    public let usage: InsightUsage
    public let skills: InsightSkills
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
