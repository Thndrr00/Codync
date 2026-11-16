import CodyncKit
import Foundation

/// The computer-wide ceiling on local mini-apps, with how many it holds, how many bots made them
/// and the values it offers.
struct LocalApps: Decodable, Sendable {
    let count: Int
    let limit: Int
    let choices: [Int]
    let botsWithApps: Int
}

extension BotStore {
    /// The saved ceiling, the apps this computer holds and the values the computer offers.
    func localApps() async throws -> LocalApps {
        try await ready().call("localApps")
    }

    /// Saves a new ceiling; throws the computer's reason when the value isn't one of them.
    func setLocalAppsLimit(_ limit: Int) async throws {
        struct Saved: Decodable, Sendable {
            let limit: Int
        }
        let _: Saved = try await ready().call("setLocalAppsLimit", ["limit": limit])
    }
}
