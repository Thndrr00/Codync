import Foundation

/// One memory entry.
public struct MemoryEntry: Decodable, Identifiable, Hashable, Sendable {
    public var id: Int64
    /// The bot whose own layer holds it; nil for what all bots share.
    public var bot: String?
    /// active · archived
    public var state: String
    public var text: String
    /// user · bot · dream · imported
    public var author: String
    public var authorBot: String?
    public var pinned: Bool
    public var updatedAt: Int64

    public var archived: Bool { state == "archived" }
}

public struct MemoryLayer: Decodable, Hashable, Sendable {
    public var entries: [MemoryEntry]
    public var used: Int
    public var cap: Int
}

/// One dreaming run: what it added and archived (`undone` when an older app undid it).
public struct DreamRun: Decodable, Identifiable, Hashable, Sendable {
    public struct Change: Decodable, Hashable, Sendable {
        public var id: Int64
        public var text: String
    }
    /// A long-form file the run added a line to.
    public struct FileNote: Decodable, Hashable, Sendable {
        public var name: String
        public var shared: Bool
        public var version: Int64
    }
    public var id: Int64
    public var at: Int64
    public var undone: Bool
    public var added: [Change]
    public var archived: [Change]
    public var files: [FileNote]?
}

/// Memory several bots share: a project's, or the Fleet one, which stands over the memory every bot used to share.
public struct Brain: Decodable, Identifiable, Hashable, Sendable {
    public var id: String
    public var name: String
    /// The Fleet brain: its layer is the old shared-memory folder.
    public var shared: Bool
    /// The bots working in it; only on the brains list.
    public var members: [String]?
}

/// A bot's memory as its screen shows it.
public struct MemoryView: Decodable, Hashable, Sendable {
    public struct Project: Decodable, Hashable, Sendable {
        public var path: String
        public var text: String
    }
    public struct Total: Decodable, Hashable, Sendable {
        public var used: Int
        public var cap: Int
    }
    /// Whether this bot uses a brain to share memory (the Fleet one included).
    public var shared: Bool
    /// The brain it works in; nil when it works in none.
    public var brain: Brain?
    /// The brain's memory ("About you" in the Fleet brain, the project's otherwise); nil without a brain.
    public var yours: MemoryLayer?
    public var bot: MemoryLayer
    /// Tokens used against the two caps together.
    public var total: Total
    public var dreams: [DreamRun]
    /// Claude Code's own notes for the bot's project folder, read-only.
    public var projectMemory: Project?
    /// The long-form files of both layers.
    public var files: MemoryFiles?
}

/// The text a single change of a composer's draft inserted, when it is long enough that the user pasted it
/// rather than typed it: typing, predictions and dictation add a word at a time.
public func pastedInsertion(from old: String, to new: String, minimum: Int = 24) -> String? {
    let old = Array(old), new = Array(new)
    guard new.count - old.count >= minimum else { return nil }
    var prefix = 0
    while prefix < old.count, old[prefix] == new[prefix] { prefix += 1 }
    var suffix = 0
    while suffix < old.count - prefix, old[old.count - 1 - suffix] == new[new.count - 1 - suffix] { suffix += 1 }
    let inserted = String(new[prefix..<(new.count - suffix)])
    return inserted.count >= minimum ? inserted : nil
}

/// A brain as one JSON file: its name and its files, so another computer can hold the same memory.
public struct BrainBundle: Codable, Hashable, Sendable {
    public struct File: Codable, Hashable, Sendable {
        public var name: String
        public var content: String
    }
    public struct Named: Codable, Hashable, Sendable {
        public var name: String
    }
    public var format: String
    public var version: Int
    public var brain: Named
    public var files: [File]
}

extension HostClient {
    public func memory(botId: String) async throws -> MemoryView {
        try await call("memory", ["botId": botId])
    }

    /// Every brain with the bots in it, the Fleet one first.
    public func brains() async throws -> [Brain] {
        struct Reply: Decodable { var brains: [Brain] }
        let reply: Reply = try await call("brains", Empty())
        return reply.brains
    }

    /// A brain made; with `id`, the one renamed.
    public func saveBrain(name: String, id: String?) async throws -> [Brain] {
        struct Reply: Decodable { var brains: [Brain] }
        let reply: Reply = if let id {
            try await call("renameBrain", ["id": id, "name": name])
        } else {
            try await call("createBrain", ["name": name])
        }
        return reply.brains
    }

    /// A bot moved into a brain (the Fleet one is the shared memory).
    public func joinBrain(botId: String, brainId: String) async throws -> [Brain] {
        struct Reply: Decodable { var brains: [Brain] }
        let reply: Reply = try await call("joinBrain", ["botId": botId, "brainId": brainId])
        return reply.brains
    }

    /// A bot taken out of its brain; it keeps its own memory and the shared one.
    public func leaveBrain(botId: String) async throws -> [Brain] {
        struct Reply: Decodable { var brains: [Brain] }
        let reply: Reply = try await call("leaveBrain", ["botId": botId])
        return reply.brains
    }

    /// A brain deleted; its bots go back to the Fleet brain and what it held is gone.
    public func deleteBrain(id: String) async throws -> [Brain] {
        struct Reply: Decodable { var brains: [Brain] }
        let reply: Reply = try await call("deleteBrain", ["id": id])
        return reply.brains
    }

    /// A brain saved as a file to give another copy of Codync.
    public func exportBrain(id: String) async throws -> BrainBundle {
        try await call("exportBrain", ["id": id])
    }

    /// A brain made from a file this app saved; `name` renames it on the way in. Answers the brains and the files
    /// the checks turned away.
    public func importBrain(_ bundle: BrainBundle, name: String?) async throws -> (brains: [Brain], skipped: Int) {
        struct Body: Encodable {
            var bundle: BrainBundle
            var name: String?
        }
        struct Reply: Decodable {
            struct Skipped: Decodable { var name: String }
            var brains: [Brain]
            var skipped: [Skipped]
        }
        let reply: Reply = try await call("importBrain", Body(bundle: bundle, name: name))
        return (reply.brains, reply.skipped.count)
    }
}
