import Foundation

public enum RosterFilter: String, CaseIterable, Sendable {
    case all, unread, attention, working, pinned

    public var title: String {
        switch self {
        case .all: "All"
        case .unread: "Unread"
        case .attention: "Needs attention"
        case .working: "Working"
        case .pinned: "Pinned"
        }
    }

    public func matches(_ bot: Bot, query: String) -> Bool {
        let selected: Bool = switch self {
        case .all: true
        case .unread: bot.unread > 0
        case .attention: bot.needsInput || bot.status == "error"
        case .working: bot.status == "working"
        case .pinned: bot.pinned
        }
        return selected && query.split(whereSeparator: \.isWhitespace).allSatisfy { term in
            [bot.name, bot.description, bot.cwd, bot.backend].contains { $0.localizedCaseInsensitiveContains(String(term)) }
        }
    }
}

public extension Bot {
    var rosterProject: String {
        if isGroup { return "Group chats" }
        return cwd.isEmpty ? "No project" : cwd
    }

    var rosterFast: String? {
        guard !isGroup, fastMode else { return nil }
        return switch fastModeSupport {
        case "offered": "Fast"
        case "refused": "Fast refused"
        case "unsupported": "Fast unavailable"
        default: "Fast requested"
        }
    }

    func rosterElapsed(at now: Date) -> String {
        let milliseconds = now.timeIntervalSince1970 * 1000
        guard let startedAt, startedAt > 0, Double(startedAt) <= milliseconds else { return "Duration unavailable" }
        let seconds = Int((milliseconds - Double(startedAt)) / 1000)
        return seconds < 3600 ? String(format: "%d:%02d", seconds / 60, seconds % 60)
            : String(format: "%d:%02d:%02d", seconds / 3600, seconds / 60 % 60, seconds % 60)
    }
}

public struct RosterSearchHit: Decodable, Identifiable, Hashable, Sendable {
    public let id: String
    public let botId: String
    public let threadId: String?
    public let text: String
    public let seq: Int64
}

public struct RosterSearchPage: Decodable, Sendable {
    public let results: [RosterSearchHit]
    public let next: Int64?
}
