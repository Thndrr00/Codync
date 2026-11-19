import Foundation

/// Behaviors brought from agentes.chat. The host owns them; this extension only calls and decodes.

public struct PresenceStatus: Decodable, Sendable {
    public let text: String
    public let until: Int64?
}

public struct SetupStep: Decodable, Sendable {
    public let id: String
    public let done: Bool
    public let label: String
}

public struct SetupChecklist: Decodable, Sendable {
    public let steps: [SetupStep]
    public let done: Int
    public let total: Int
    public let complete: Bool
}

public struct StretchCard: Decodable, Sendable {
    public let type: String
    public let agent: String
    public let reason: String
    public let action: String
    public let bodyCode: String
}

public struct ApprovalRequest: Codable, Sendable {
    public let type: String
    public let requestId: String
    public let agent: String
    public let reason: String
    public let options: [ApprovalOption]
    public let status: String
    public let selected: String?
}

public struct ApprovalOption: Codable, Sendable {
    public let id: String
    public let label: String
    public let kind: String
}

public struct VaultEntry: Decodable, Sendable {
    public let name: String
    public let source: String
    public let masked: String
}

public extension HostClient {
    func presence(botId: String) async throws -> PresenceStatus? {
        struct Res: Decodable { var status: PresenceStatus? }
        return try await call("botPresence", ["botId": botId]).status
    }

    func setPresence(botId: String, status: String, until: Int64? = nil) async throws -> PresenceStatus? {
        struct Body: Encodable { let botId: String; let status: String; let until: Int64? }
        struct Res: Decodable { var status: PresenceStatus? }
        return try await call("botPresence", Body(botId: botId, status: status, until: until)).status
    }

    func setup(botId: String) async throws -> SetupChecklist {
        try await call("botSetup", ["botId": botId])
    }

    func stretchCard(agent: String, reason: String) async throws -> StretchCard {
        try await call("stretchCard", ["agent": agent, "reason": reason])
    }

    func approvalCard(agent: String, reason: String, requestId: String) async throws -> ApprovalRequest {
        try await call("agentApproval", ["agent": agent, "reason": reason, "requestId": requestId])
    }

    func decideApproval(_ card: ApprovalRequest, option: String) async throws -> ApprovalRequest {
        struct Body: Encodable { let op: String; let card: ApprovalRequest; let option: String }
        return try await call("agentApproval", Body(op: "decide", card: card, option: option))
    }

    func vaultMask(name: String, secret: String, source: String = "keychain") async throws -> VaultEntry {
        try await call("vaultMask", ["name": name, "secret": secret, "source": source])
    }
}
