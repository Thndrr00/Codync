import CodyncKit
import Foundation

extension BotStore {
    /// What the bot's `codync-docs` index holds: the folder it reads, how many files and bytes, and whether
    /// a limit cut it short. Hosts before the feature answer "unknown method".
    public func docsStatus(botId: String) async throws -> DocsStatus {
        let client = try await ready()
        return try await client.call("docsStatus", ["botId": botId], timeout: 60)
    }

    /// Names the folder the bot's index reads; an empty path clears the choice, so its working folder is
    /// indexed again. Answers with the index the new folder gives. The first read walks the folder, which
    /// can take a while on a large repository.
    public func setDocsFolder(_ path: String, botId: String) async throws -> DocsStatus {
        let client = try await ready()
        return try await client.call("setDocsFolder", ["botId": botId, "path": path], timeout: 60)
    }
}
