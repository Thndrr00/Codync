import Foundation

// The phone reaches the host's bdots features through the same `call` bridge every other
// screen uses (see HostClient+Artifacts). No second transport, no client-side persistence:
// the host owns apps, open items and bookmarks; this extension only decodes its answers.

/// One mini-app request answered by the host on the device's behalf. Phones cannot reach the
/// host's loopback `/apps/...` server, so `appRequest` runs that request inside the host.
public struct AppResponse: Sendable {
    public var status: Int
    public var contentType: String
    /// The sandbox policy the host served with the page. Web views must keep it.
    public var csp: String
    public var body: Data
}

public struct AppRequestWire: Decodable, Sendable {
    var status: Int
    var contentType: String
    var csp: String
    var body: String
}

public extension HostClient {
    /// The bot's mini-apps with their share links and collection counts.
    func listBotApps(botId: String) async throws -> [BotApp] {
        struct Res: Decodable { var apps: [BotApp] }
        let res: Res = try await call("listBotApps", ["botId": botId])
        return res.apps
    }

    /// Issues a view or edit capability link. The returned URL is ready to share.
    func createAppLink(botId: String, slug: String, label: String, mode: AppLinkMode) async throws -> CreatedAppLink {
        struct Body: Encodable { let botId: String; let slug: String; let label: String; let mode: AppLinkMode }
        return try await call(
            "createAppLink",
            Body(botId: botId, slug: slug, label: label, mode: mode)
        )
    }

    func revokeAppLink(botId: String, slug: String, token: String) async throws -> Bool {
        struct Res: Decodable { var revoked: Bool }
        let res: Res = try await call("revokeAppLink", ["botId": botId, "slug": slug, "token": token])
        return res.revoked
    }

    func deleteBotApp(botId: String, slug: String) async throws -> Bool {
        struct Res: Decodable { var deleted: Bool }
        let res: Res = try await call("deleteBotApp", ["botId": botId, "slug": slug])
        return res.deleted
    }

    /// Open items the bot is tracking, newest first.
    func listOpenItems(botId: String) async throws -> [OpenItem] {
        struct Res: Decodable { var items: [OpenItem] }
        let res: Res = try await call("listOpenItems", ["botId": botId])
        return res.items
    }

    func listBookmarks(botId: String) async throws -> [MessageBookmark] {
        struct Res: Decodable { var bookmarks: [MessageBookmark] }
        let res: Res = try await call("listBookmarks", ["botId": botId])
        return res.bookmarks
    }

    func deleteBookmark(botId: String, entryId: String) async throws -> Bool {
        struct Res: Decodable { var deleted: Bool }
        let res: Res = try await call("deleteBookmark", ["botId": botId, "entryId": entryId])
        return res.deleted
    }

    /// Runs one request against a mini-app inside the host and returns its body decoded once.
    func appRequest(
        botId: String,
        slug: String,
        token: String,
        method: String = "GET",
        path: String = "",
        body: Data? = nil
    ) async throws -> AppResponse {
        struct Body: Encodable {
            let botId: String
            let slug: String
            let token: String
            let method: String
            let path: String
            let body: String
        }
        let wire: AppRequestWire = try await call("appRequest", Body(
            botId: botId,
            slug: slug,
            token: token,
            method: method,
            path: path,
            body: body.map { $0.base64EncodedString() } ?? ""
        ))
        return try Self.decodeAppResponse(wire)
    }

    /// Kept separate from the transport so the bridge can be tested with a canned host reply.
    static func decodeAppResponse(_ wire: AppRequestWire) throws -> AppResponse {
        guard let data = Data(base64Encoded: wire.body) else { throw AppBridgeError.invalidBody }
        return AppResponse(status: wire.status, contentType: wire.contentType, csp: wire.csp, body: data)
    }
}

public enum AppBridgeError: Error, Equatable {
    case invalidBody
}
