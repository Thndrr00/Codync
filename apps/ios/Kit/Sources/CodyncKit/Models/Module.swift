import Foundation

/// Opaque JSON that must survive a round trip: a module bundle the host verifies file by file.
public enum JSONValue: Codable, Hashable, Sendable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: any Decoder) throws {
        let single = try decoder.singleValueContainer()
        if single.decodeNil() {
            self = .null
        } else if let value = try? single.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? single.decode(Double.self) {
            self = .number(value)
        } else if let value = try? single.decode(String.self) {
            self = .string(value)
        } else if let value = try? single.decode([JSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try single.decode([String: JSONValue].self))
        }
    }

    public func encode(to encoder: any Encoder) throws {
        var single = encoder.singleValueContainer()
        switch self {
        case .null: try single.encodeNil()
        case .bool(let value): try single.encode(value)
        case .number(let value): try single.encode(value)
        case .string(let value): try single.encode(value)
        case .array(let value): try single.encode(value)
        case .object(let value): try single.encode(value)
        }
    }
}

/// What a module asks for, and what the owner approved of it (`host/src/modules/manifest.rs`).
public struct ModulePermissions: Codable, Hashable, Sendable {
    public var tools: [String]
    public var events: [String]
    public var network: [String]
    /// `none`, `read` or `write`: how much of a bot's folder it may touch.
    public var files: String

    public init(tools: [String] = [], events: [String] = [], network: [String] = [], files: String = "none") {
        self.tools = tools
        self.events = events
        self.network = network
        self.files = files
    }
}

/// What one call may cost; the host holds every module to it.
public struct ModuleLimits: Codable, Hashable, Sendable {
    public var callsPerMinute: Int
    public var timeMs: Int
    public var memoryMb: Int
}

public struct ModuleTool: Codable, Hashable, Sendable, Identifiable {
    public var name: String
    public var description: String
    public var id: String { name }
}

public struct ModuleViewRef: Codable, Hashable, Sendable, Identifiable {
    public var id: String
    public var title: String
}

/// A module installed on this computer: a signed folder the host runs in its own process.
public struct HostModule: Codable, Hashable, Sendable, Identifiable {
    public var id: String
    public var name: String
    public var version: String?
    public var description: String
    public var symbol: String?
    public var contract: Int?
    public var enabled: Bool
    /// Why it can't run, when its folder no longer verifies.
    public var broken: String?
    /// `running`, `stopped`, `disabled` or `broken`.
    public var state: String
    /// Why its process stopped, when it did.
    public var stopped: String?
    public var permissions: ModulePermissions?
    public var approved: ModulePermissions
    public var limits: ModuleLimits
    public var views: [ModuleViewRef]
    public var tools: [ModuleTool]
    public var installedAt: Int
    public var folder: String

    public var stateText: String {
        if broken != nil { return "Broken" }
        if !enabled { return "Off" }
        return state == "running" ? "Running" : "Ready"
    }

    /// What the owner approved, in one line.
    public var approvedText: String {
        var parts: [String] = []
        if !approved.tools.isEmpty { parts.append("\(approved.tools.count) tools") }
        if !approved.events.isEmpty { parts.append("\(approved.events.count) events") }
        if approved.files != "none" { parts.append("\(approved.files) files") }
        return parts.isEmpty ? "Nothing approved yet" : parts.joined(separator: " · ")
    }
}

/// A module's screen, as rows the apps draw with their own components.
public struct ModuleScreen: Codable, Hashable, Sendable {
    public struct Section: Codable, Hashable, Sendable {
        public struct Row: Codable, Hashable, Sendable {
            public var label: String
            public var value: String
        }

        public var title: String
        public var rows: [Row]
    }

    public var title: String
    public var sections: [Section]
}

public struct ModulesReply: Decodable, Sendable {
    public var modules: [HostModule]
}

private struct ModuleViewReply: Decodable, Sendable {
    var view: ModuleScreen
}

public extension HostClient {
    func modules() async throws -> [HostModule] {
        let reply: ModulesReply = try await call("modules")
        return reply.modules
    }

    /// A module from a folder on this computer, or from a bundle another one saved.
    func installModule(folder: String? = nil, bundle: JSONValue? = nil) async throws -> [HostModule] {
        struct Body: Encodable {
            var folder: String?
            var bundle: JSONValue?
        }
        let reply: ModulesReply = try await call("installModule", Body(folder: folder, bundle: bundle), timeout: 60)
        return reply.modules
    }

    func uninstallModule(_ id: String) async throws -> [HostModule] {
        struct Body: Encodable { var id: String }
        let reply: ModulesReply = try await call("uninstallModule", Body(id: id), timeout: 30)
        return reply.modules
    }

    func setModule(_ id: String, enabled: Bool) async throws -> [HostModule] {
        struct Body: Encodable {
            var id: String
            var enabled: Bool
        }
        let reply: ModulesReply = try await call("setModule", Body(id: id, enabled: enabled), timeout: 30)
        return reply.modules
    }

    /// `nil` approves everything the manifest asks for; a subset is cut down to what it declares.
    func approveModule(_ id: String, permissions: ModulePermissions? = nil) async throws -> [HostModule] {
        struct Body: Encodable {
            var id: String
            var permissions: ModulePermissions?
        }
        let reply: ModulesReply = try await call("approveModule", Body(id: id, permissions: permissions), timeout: 30)
        return reply.modules
    }

    /// The module as one file for another computer.
    func exportModule(_ id: String) async throws -> (name: String, bundle: JSONValue) {
        struct Body: Encodable { var id: String }
        struct Reply: Decodable {
            var name: String
            var bundle: JSONValue
        }
        let reply: Reply = try await call("exportModule", Body(id: id), timeout: 30)
        return (reply.name, reply.bundle)
    }

    func moduleView(_ id: String, view: String, botId: String? = nil) async throws -> ModuleScreen {
        struct Body: Encodable {
            var id: String
            var view: String
            var botId: String?
        }
        let reply: ModuleViewReply = try await call("moduleView", Body(id: id, view: view, botId: botId), timeout: 120)
        return reply.view
    }
}
