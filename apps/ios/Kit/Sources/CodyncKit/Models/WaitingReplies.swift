import Foundation

/// One row of the "Waiting for replies" card above a chat's composer.
public struct WaitingReply: Identifiable, Sendable {
    public enum State: String, Sendable { case asked, stopped }

    public let id: String
    public let name: String
    public let request: String
    public let state: State
    /// The notice that carries the ask, for the Stop action.
    public let entry: Entry

    public init(id: String, name: String, request: String, state: State, entry: Entry) {
        self.id = id; self.name = name; self.request = request; self.state = state; self.entry = entry
    }
}

/// Derives the outstanding "Asked" requests of a chat or thread from structured bot messages.
/// A stopped row stays in the list with its new state instead of vanishing.
public enum WaitingReplies {
    public static func rows(_ entries: [Entry], botId: String, threadId: String?,
                            stopped: Set<String>, name: (String) -> String) -> [WaitingReply] {
        var out: [WaitingReply] = []
        for entry in entries {
            guard let exchange = BotExchange(entry), exchange.chatId == botId,
                  exchange.source == botId, entry.threadId == threadId else { continue }
            let isStopped = stopped.contains(entry.id)
            let asked = (entry.data.heading ?? "").hasPrefix("Asked")
            if !isStopped && (!asked || exchange.outcome != .pending) { continue }
            out.append(WaitingReply(
                id: entry.id,
                name: exchange.peerName ?? name(exchange.target),
                request: exchange.text,
                state: isStopped ? .stopped : .asked,
                entry: entry,
            ))
        }
        return out.sorted { $0.id < $1.id }
    }
}
