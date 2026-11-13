import CodyncKit
import Foundation

/// The hourly ceiling on requests local scripts send to routines of this computer.
struct ScriptLimit: Decodable, Sendable {
    let perHour: Int
    let choices: [Int]
}

extension BotStore {
    /// The saved ceiling and the values the computer offers.
    func scriptLimit() async throws -> ScriptLimit {
        try await ready().call("scriptLimit")
    }

    /// Saves a new ceiling; throws the computer's reason when the value isn't one of them.
    func setScriptLimit(_ perHour: Int) async throws {
        struct Saved: Decodable, Sendable {
            let perHour: Int
        }
        let _: Saved = try await ready().call("setScriptLimit", ["perHour": perHour])
    }
}
