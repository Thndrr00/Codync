import Foundation

/// A read-only summary of one bot, computed by the host from its own data and the bdots helpers.
public struct BotInsights: Decodable, Sendable {
    public let botId: String
    public let model: String
    public let provider: String?
    public let routines: [InsightRoutine]
    public let usage: InsightUsage
    public let skills: InsightSkills
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
