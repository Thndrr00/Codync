import Foundation

/// Whose side of a chat a search keeps; `everyone` sends no filter.
public enum ChatSearchAuthor: String, CaseIterable, Sendable {
    case everyone
    case me
    case bot

    public var title: String {
        switch self {
        case .everyone: "Everyone"
        case .me: "Me"
        case .bot: "Bot"
        }
    }

    /// What the host reads (`author`).
    public var wire: String? {
        switch self {
        case .everyone: nil
        case .me: "user"
        case .bot: "agent"
        }
    }
}

/// Where in a chat a search keeps its words: everywhere, the chat itself, or its side conversations; `everything`
/// sends no filter.
public enum ChatSearchScope: String, CaseIterable, Sendable {
    case everything
    case thisChat
    case threads

    public var title: String {
        switch self {
        case .everything: "Everything"
        case .thisChat: "This chat"
        case .threads: "Threads"
        }
    }

    /// What the host reads (`channel`).
    public var wire: String? {
        switch self {
        case .everything: nil
        case .thisChat: "main"
        case .threads: "threads"
        }
    }
}

/// One match of a search inside a chat: a message with the snippet around it, a page, or a memory note.
public struct ChatSearchHit: Decodable, Identifiable, Hashable, Sendable {
    public enum Kind: String, Decodable, Sendable {
        case message
        case document
        case memory
    }

    public let kind: Kind
    public let id: String
    /// When it was said (epoch milliseconds).
    public let at: Int64
    /// The text around the match, the matched words in brackets.
    public let snippet: String
    public let entryKind: String?
    public let author: String?
    /// A page's title or a note's name.
    public let title: String?
    public let threadId: String?
    /// A note of the memory every bot shares, rather than the bot's own.
    public let shared: Bool?

    public var isPage: Bool { kind == .document }
    public var isNote: Bool { kind == .memory }
    public var isMine: Bool { entryKind == "user" }
}

/// What a search inside a chat answered: its matches, best first.
public struct ChatSearchPage: Decodable, Sendable {
    public let hits: [ChatSearchHit]
}
