import Foundation

/// Where a mission is: proposed until somebody takes it on, then running, waiting on something,
/// finished, or dropped with a reason.
public enum MissionStatus: String, Codable, Sendable, CaseIterable {
    case proposed, active, waiting, done, abandoned

    /// A finished or dropped mission: it stays in the list only when asked for.
    public var isReturned: Bool {
        self == .done || self == .abandoned
    }

    /// The status somebody pushing this mission on moves it to, if there is such a move.
    public var next: MissionStatus? {
        switch self {
        case .proposed, .waiting, .done, .abandoned: .active
        case .active: .waiting
        }
    }

    /// The verb of that move.
    public var nextTitle: String {
        switch self {
        case .proposed: L10n.tr("Activate")
        case .active: L10n.tr("Pause")
        case .waiting: L10n.tr("Resume")
        case .done, .abandoned: L10n.tr("Reopen")
        }
    }

    public var label: String {
        switch self {
        case .proposed: L10n.tr("Proposed")
        case .active: L10n.tr("Running")
        case .waiting: L10n.tr("Waiting")
        case .done: L10n.tr("Done")
        case .abandoned: L10n.tr("Abandoned")
        }
    }
}

public enum MissionStepStatus: String, Codable, Sendable {
    case pending, active, done, skipped

    /// The glyph a step shows as.
    public var mark: String {
        switch self {
        case .pending: "·"
        case .active: "▸"
        case .done: "✓"
        case .skipped: "–"
        }
    }
}

public struct MissionStep: Decodable, Identifiable, Sendable {
    public let id: String
    public let missionId: String
    public let position: Int64
    public let text: String
    public let status: MissionStepStatus
    public let at: Int64
}

/// An objective that outlives a turn: what is done, what is left, and why it stopped.
public struct Mission: Decodable, Identifiable, Sendable {
    public let id: String
    public let botId: String
    public let title: String
    public let goal: String
    public let status: MissionStatus
    public let stopReason: String?
    public let checkpoint: String
    public let runId: String?
    public let approvalId: String?
    public let createdAt: Int64
    public let updatedAt: Int64
    public let steps: [MissionStep]
}
