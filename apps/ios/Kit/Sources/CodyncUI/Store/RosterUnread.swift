import CodyncKit
import Foundation

/// The host payload behind a roster "Mark unread" menu item.
public struct UnreadAction: Equatable, Sendable {
    public let entryId: String

    public init(entryId: String) {
        self.entryId = entryId
    }
}

/// The pure decision behind "Mark unread": the host needs a message to start from,
/// so without one there is no menu item to offer.
public func unreadAction(_ entryId: String?) -> UnreadAction? {
    guard let entryId else { return nil }
    return UnreadAction(entryId: entryId)
}

/// Which message a roster "Mark unread" starts from: the newest chat-visible agent
/// reply in the cached conversation. The host marks it and everything after it
/// unread; opening the chat clears them again.
public func unreadEntryId(_ entries: [Entry]) -> String? {
    for entry in entries.reversed() where entry.kind == "agent" && entry.isChat {
        return entry.id
    }
    return nil
}
