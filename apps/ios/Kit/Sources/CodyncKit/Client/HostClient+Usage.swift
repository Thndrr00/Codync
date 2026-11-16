import Foundation

public extension HostClient {
    func usage(refresh: Bool = false) async throws -> Usage {
        try await call("usage", ["refresh": refresh])
    }

    /// How often each skill's text entered a turn (`skillUsage`); an older host refuses the call.
    func skillUsage() async throws -> SkillUsage {
        try await call("skillUsage")
    }

    /// The computer's memory, CPU and disk (`hostHealth`); an older host refuses the call.
    func hostHealth() async throws -> HostHealth {
        try await call("hostHealth")
    }

    /// Whether this computer lists one agent's limits.
    func setUsageShown(_ id: String, shown: Bool) async throws -> Usage {
        struct Body: Encodable { var id: String; var shown: Bool }
        return try await call("setUsageShown", Body(id: id, shown: shown))
    }

    func setUsageOrder(_ ids: [String]) async throws -> Usage {
        try await call("setUsageOrder", ["ids": ids])
    }
}
