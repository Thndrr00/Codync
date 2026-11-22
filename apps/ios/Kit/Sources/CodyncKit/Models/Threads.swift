import Foundation

/// A thread of a chat: a side conversation opened from a message, so one point can be followed
/// without filling the chat. Lists carry only where a thread ends; its lines come from `thread`.
public struct ThreadInfo: Codable, Identifiable, Hashable, Sendable {
    public var id: String
    public var title: String
    /// The message the thread was branched from.
    public var fromEntry: String?
    /// That message's text, for "Branched from …".
    public var branchText: String?
    public var archived: Bool
    public var createdAt: Int64
    public var updatedAt: Int64
    public var unread: Int
    public var lastText: String?
    public var lastAuthor: String?

    public init(id: String, title: String, fromEntry: String?, branchText: String?, archived: Bool,
                createdAt: Int64, updatedAt: Int64, unread: Int, lastText: String?, lastAuthor: String?) {
        self.id = id
        self.title = title
        self.fromEntry = fromEntry
        self.branchText = branchText
        self.archived = archived
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.unread = unread
        self.lastText = lastText
        self.lastAuthor = lastAuthor
    }

    /// The state a list row shows beside the thread.
    public var status: ThreadStatus {
        ThreadStatus.of(unread: unread, archived: archived, lastAuthor: lastAuthor)
    }

    /// What a list row calls the thread: its own title, or the message it came from.
    public var name: String {
        let named = title.trimmingCharacters(in: .whitespacesAndNewlines)
        if !named.isEmpty { return named }
        guard let branch = branchText, !branch.isEmpty else { return L10n.tr("Thread") }
        return String(branch.replacingOccurrences(of: "\n", with: " ").prefix(60))
    }
}

/// A thread's state in a list, derived from what the host already sends (no new endpoint): a
/// thread with unread lines waits for you, one whose last line is the bot's has been answered.
public enum ThreadStatus: String, Sendable {
    case waiting
    case answered
    case open
    case archived

    /// Pure so the list's status can be tested without a host, a store or a view.
    public static func of(unread: Int, archived: Bool, lastAuthor: String?) -> ThreadStatus {
        if archived { return .archived }
        if unread > 0 { return .waiting }
        if let lastAuthor, !lastAuthor.isEmpty, lastAuthor != "user" { return .answered }
        return .open
    }

    public var label: String {
        switch self {
        case .waiting: L10n.tr("Waiting on you")
        case .answered: L10n.tr("Answered")
        case .open: L10n.tr("Open")
        case .archived: L10n.tr("Archived")
        }
    }
}

/// One thread as `thread` answers it: its lines, its info, and its unread count before reading.
public struct ThreadRead: Decodable, Sendable {
    public let entries: [Entry]
    public let thread: ThreadInfo?
    public let unread: Int
}
