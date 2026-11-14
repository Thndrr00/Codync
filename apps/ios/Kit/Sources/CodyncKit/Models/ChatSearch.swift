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

/// One match of a search inside a chat: a message with the snippet around it, or a page.
public struct ChatSearchHit: Decodable, Identifiable, Hashable, Sendable {
    public enum Kind: String, Decodable, Sendable {
        case message
        case document
    }

    public let kind: Kind
    public let id: String
    /// When it was said (epoch milliseconds).
    public let at: Int64
    /// The text around the match, the matched words in brackets.
    public let snippet: String
    public let entryKind: String?
    public let author: String?
    /// A page's title.
    public let title: String?
    public let threadId: String?

    public var isPage: Bool { kind == .document }
    public var isMine: Bool { entryKind == "user" }
}

/// What a search inside a chat answered: its matches, best first.
public struct ChatSearchPage: Decodable, Sendable {
    public let hits: [ChatSearchHit]
}
