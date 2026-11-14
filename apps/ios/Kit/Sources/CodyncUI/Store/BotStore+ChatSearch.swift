import CodyncKit
import Foundation

extension BotStore {
    /// What this chat knows about `query`: its messages, its pages and its memory, best match first.
    func searchChat(_ query: String, botId: String, author: ChatSearchAuthor, scope: ChatSearchScope, tools: Bool,
                    limit: Int = 30) async throws -> [ChatSearchHit] {
        struct Request: Encodable {
            let botId: String
            let query: String
            let limit: Int
            let tools: Bool
            let author: String?
            let channel: String?
        }
        let client = try await ready()
        let response: ChatSearchPage = try await client.call(
            "search",
            Request(botId: botId, query: query, limit: limit, tools: tools, author: author.wire, channel: scope.wire))
        try Task.checkCancellation()
        return response.hits
    }

    /// The messages around a hit, for reading it in place.
    func chatHitContext(botId: String, entryId: String) async throws -> [Entry] {
        struct Response: Decodable { let entries: [Entry] }
        let client = try await ready()
        let response: Response = try await client.call("messageContext", ["botId": botId, "entryId": entryId])
        try Task.checkCancellation()
        return response.entries.visibleEntries()
    }

    /// A page a search found.
    func searchPage(_ id: String) async throws -> (title: String, body: String) {
        struct Document: Decodable { let title: String; let body: String? }
        struct Response: Decodable { let document: Document }
        let client = try await ready()
        let response: Response = try await client.call("getDocument", ["id": id])
        try Task.checkCancellation()
        return (response.document.title, response.document.body ?? "")
    }

    /// A memory note a search found.
    func searchNote(botId: String, file: String, shared: Bool) async throws -> String {
        let client = try await ready()
        let detail = try await client.memoryFile(botId: botId, file: file, shared: shared)
        try Task.checkCancellation()
        return detail.content ?? ""
    }
}
