import CodyncKit
import Foundation

/// One card on the board (host/src/store/board.rs): what the person is doing, and the bot that owns it.
public struct BoardTask: Decodable, Identifiable, Equatable, Sendable {
    public static let statuses = ["todo", "doing", "waiting", "done"]

    public var id: String
    public var title: String
    public var detail: String
    public var status: String
    public var botId: String?
    public var routineId: String?
    public var dueAt: Int?
    /// `daily` or `weekly`: finishing the card creates the next one.
    public var repeat: String?
    public var createdAt: Int
    public var updatedAt: Int

    public var isDone: Bool { status == "done" }
    public var isRepeating: Bool { repeat != nil }
}

/// A group saved as a reusable team, with its members' names.
public struct TeamTemplate: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var name: String
    public var description: String
    public var createdAt: Int
    public var mode: String
    public var voice: String
    public var members: [String]
}

/// A page this computer serves to anyone with the link.
public struct SharedLink: Decodable, Identifiable, Equatable, Sendable {
    public var token: String
    public var title: String
    public var createdAt: Int
    public var expiresAt: Int?
    public var bytes: Int
    public var url: String

    public var id: String { token }
}

/// One group member's turn in a short meeting; `audio` is base64 for `audioFormat`.
public struct MeetingTurn: Decodable, Identifiable, Equatable, Sendable {
    public var botId: String
    public var name: String
    public var text: String
    public var audio: String?
    public var audioFormat: String?

    public var id: String { botId }
}

public struct TeamMeeting: Decodable, Equatable, Sendable {
    public var topic: String
    public var turns: [MeetingTurn]
    public var minutes: String
}

extension BotStore {
    // MARK: board

    public func boardTasks() async throws -> [BoardTask] {
        struct Wrap: Decodable { var tasks: [BoardTask] }
        let client = try await ready()
        let wrap: Wrap = try await client.call("boardTasks")
        return wrap.tasks
    }

    @discardableResult
    public func saveBoardTask(title: String, detail: String = "", status: String = "todo",
                              botId: String? = nil, dueAt: Int? = nil, repeat: String? = nil) async throws -> BoardTask {
        struct Wrap: Decodable { var task: BoardTask }
        var body: [String: any Encodable] = ["title": title, "detail": detail, "status": status]
        if let botId { body["botId"] = botId }
        if let dueAt { body["dueAt"] = dueAt }
        if let repeat { body["repeat"] = repeat }
        let client = try await ready()
        let wrap: Wrap = try await client.call("saveBoardTask", AnyEncodable(body))
        return wrap.task
    }

    @discardableResult
    public func moveBoardTask(_ id: String, to status: String) async throws -> Bool {
        struct Wrap: Decodable { var moved: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("moveBoardTask", ["id": id, "status": status])
        return wrap.moved
    }

    @discardableResult
    public func deleteBoardTask(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var deleted: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("deleteBoardTask", ["id": id])
        return wrap.deleted
    }

    // MARK: teams

    public func teamTemplates() async throws -> [TeamTemplate] {
        struct Wrap: Decodable { var templates: [TeamTemplate] }
        let client = try await ready()
        let wrap: Wrap = try await client.call("teamTemplates")
        return wrap.templates
    }

    /// Adds the saved team as a new group on this computer.
    @discardableResult
    public func installTeamTemplate(_ id: String) async throws -> Bot {
        struct Wrap: Decodable { var bot: Bot }
        let client = try await ready()
        let wrap: Wrap = try await client.call("installTeamTemplate", ["id": id])
        return wrap.bot
    }

    // MARK: shared links

    public func sharedLinks() async throws -> [SharedLink] {
        struct Wrap: Decodable { var links: [SharedLink] }
        let client = try await ready()
        let wrap: Wrap = try await client.call("sharedLinks")
        return wrap.links
    }

    @discardableResult
    public func revokeSharedLink(_ token: String) async throws -> Bool {
        struct Wrap: Decodable { var revoked: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("revokeShare", ["token": token])
        return wrap.revoked
    }

    // MARK: meetings

    /// A short roundtable of the group's members on `topic`; `voice` also has each turn spoken.
    public func teamMeeting(groupId: String, topic: String, voice: Bool = false) async throws -> TeamMeeting {
        let client = try await ready()
        let body = AnyEncodable(["botId": groupId, "topic": topic, "voice": voice])
        return try await client.call("teamMeeting", body, timeout: 180)
    }
}

/// A dictionary whose values are encodable, so a body can leave out nil fields.
struct AnyEncodable: Encodable {
    private let encodeValue: (Encoder) throws -> Void

    init(_ values: [String: any Encodable]) {
        encodeValue = { encoder in
            var container = encoder.container(keyedBy: Key.self)
            for (name, value) in values {
                try container.encode(AnyValue(value), forKey: Key(name))
            }
        }
    }

    func encode(to encoder: Encoder) throws { try encodeValue(encoder) }

    private struct Key: CodingKey {
        let stringValue: String
        var intValue: Int? { nil }
        init(_ name: String) { stringValue = name }
        init?(stringValue: String) { self.init(stringValue) }
        init?(intValue: Int) { nil }
    }

    private struct AnyValue: Encodable {
        let value: any Encodable
        init(_ value: any Encodable) { self.value = value }
        func encode(to encoder: Encoder) throws { try value.encode(to: encoder) }
    }
}
