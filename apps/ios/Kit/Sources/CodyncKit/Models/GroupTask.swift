import Foundation

/// One row of a group's request tree (`chat/group/tasks.rs`): the request, or a member's assignment under it.
/// Field names match the host's camelCase JSON.
public struct GroupTask: Codable, Identifiable, Hashable, Sendable {
    public var id: String
    public var groupId: String
    /// The lane's key: the chat, or its thread.
    public var lane: String
    /// The user's message the tree answers.
    public var requestId: String
    public var rootId: String
    public var parentId: String?
    /// The member the row belongs to; nil for the request of a channel with no lead.
    public var ownerId: String?
    public var ownerName: String?
    /// `request` or `assignment`.
    public var kind: String
    /// `queued`, `running`, `completed`, `failed`, `paused` or `cancelled`.
    public var state: String
    public var instruction: String
    /// Why it stopped or failed.
    public var note: String?
    public var createdAt: Int64
    public var updatedAt: Int64

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        groupId = try c.decodeIfPresent(String.self, forKey: .groupId) ?? ""
        lane = try c.decodeIfPresent(String.self, forKey: .lane) ?? ""
        requestId = try c.decodeIfPresent(String.self, forKey: .requestId) ?? ""
        rootId = try c.decodeIfPresent(String.self, forKey: .rootId) ?? ""
        parentId = try c.decodeIfPresent(String.self, forKey: .parentId)
        ownerId = try c.decodeIfPresent(String.self, forKey: .ownerId)
        ownerName = try c.decodeIfPresent(String.self, forKey: .ownerName)
        kind = try c.decodeIfPresent(String.self, forKey: .kind) ?? "assignment"
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "queued"
        instruction = try c.decodeIfPresent(String.self, forKey: .instruction) ?? ""
        note = try c.decodeIfPresent(String.self, forKey: .note)
        createdAt = try c.decodeIfPresent(Int64.self, forKey: .createdAt) ?? 0
        updatedAt = try c.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? createdAt
    }

    /// Still part of the request's work: it may take a turn, or wait for one.
    public var isOpen: Bool { ["queued", "running", "paused", "failed"].contains(state) }
    /// Stopped or failed: it offers Continue, Reassign and Cancel.
    public var isStopped: Bool { state == "paused" || state == "failed" }
}
