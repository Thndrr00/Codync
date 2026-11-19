import CodyncKit
import Foundation

extension BotStore {
    /// The computer's ceiling on local mini-apps, how many it holds, the values it offers and how many
    /// bots made them.
    public func localApps() async throws -> LocalApps {
        let client = try await ready()
        return try await client.call("localApps")
    }

    /// Saves a new ceiling; the computer keeps the old one and says why when the value isn't one of its own.
    public func setLocalAppsLimit(_ limit: Int) async throws {
        let client = try await ready()
        let _: Empty = try await client.call("setLocalAppsLimit", ["limit": limit])
    }
}
