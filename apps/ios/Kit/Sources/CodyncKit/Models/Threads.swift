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
    /// What kept the thread besides the drawer: `dictation` for a kept dictation.
    public var source: String?
    /// For a dictation, the chat message it was dictated under.
    public var sourceEntry: String?
    /// That message's text, so the thread says where the dictation came from.
    public var sourceText: String?
    public var archived: Bool
    public var createdAt: Int64
    public var updatedAt: Int64
    public var unread: Int
    public var lastText: String?
    public var lastAuthor: String?

    public init(id: String, title: String, fromEntry: String?, branchText: String?, archived: Bool,
                createdAt: Int64, updatedAt: Int64, unread: Int, lastText: String?, lastAuthor: String?,
                source: String? = nil, sourceEntry: String? = nil, sourceText: String? = nil) {
        self.id = id
        self.title = title
        self.fromEntry = fromEntry
        self.branchText = branchText
        self.source = source
        self.sourceEntry = sourceEntry
        self.sourceText = sourceText
        self.archived = archived
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.unread = unread
        self.lastText = lastText
        self.lastAuthor = lastAuthor
    }

    /// What a list row calls the thread: its own title, or the message it came from.
    public var name: String {
        let named = title.trimmingCharacters(in: .whitespacesAndNewlines)
        if !named.isEmpty { return named }
        guard let branch = branchText, !branch.isEmpty else { return L10n.tr("Thread") }
        return String(branch.replacingOccurrences(of: "\n", with: " ").prefix(60))
    }
}

/// One thread as `thread` answers it: its lines, its info, and its unread count before reading.
public struct ThreadRead: Decodable, Sendable {
    public let entries: [Entry]
    public let thread: ThreadInfo?
    public let unread: Int
}
