import Foundation

/// Who pinned a message: a person (their device, or none at the computer itself) or the chat's agent.
public struct Pinner: Codable, Hashable, Sendable {
    public var kind: String
    public var device: String?
    public var botId: String?

    public init(kind: String, device: String? = nil, botId: String? = nil) {
        self.kind = kind
        self.device = device
        self.botId = botId
    }
}

/// Who pinned a message, and when: it is in the chat's pin list. Rides on the entry, so its rev carries it.
public struct Pinned: Codable, Hashable, Sendable {
    public var by: Pinner
    public var at: Int64
}

/// A pinned message of a chat or one of its threads.
public struct PinnedMessage: Codable, Hashable, Sendable, Identifiable {
    public var messageId: String
    /// The side thread the message is in, or nil for the chat itself.
    public var threadId: String?
    public var threadTitle: String?
    public var pinnedAt: Int64
    public var pinnedBy: Pinner
    /// Who wrote it: a person, or this chat's agent.
    public var author: String
    public var authorBotId: String?
    public var createdAt: Int64
    /// The start of the message, or the names of its files when it has no text.
    public var text: String

    public var id: String { messageId }
}
