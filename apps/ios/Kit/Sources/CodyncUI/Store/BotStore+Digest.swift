import CodyncKit
import Foundation

public struct DigestApproval: Decodable, Equatable, Sendable {
    public var bot: String
    public var title: String
}

public struct DigestEvent: Decodable, Equatable, Sendable {
    public var title: String?
    public var start: Int?
    public var allDay: Bool?
}

/// The host's brief for today: what is on the board, what waits for an answer, the calendar and what
/// the person is waiting on. Computed without a model.
public struct DailyDigest: Decodable, Equatable, Sendable {
    public var text: String
    public var overdue: [String]
    public var dueToday: [String]
    public var inProgress: [String]
    public var approvals: [DigestApproval]
    public var events: [DigestEvent]
    public var waiting: [String]
}

extension BotStore {
    public func dailyDigest() async throws -> DailyDigest {
        let client = try await ready()
        return try await client.call("dailyDigest", Empty(), timeout: 60)
    }
}
