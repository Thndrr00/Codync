import Foundation

/// The step a bot stopped at for the person (a CAPTCHA, a login, a payment).
public struct TakeoverRequest: Codable, Hashable, Sendable {
    /// waiting · taken · resolved · cancelled · expired
    public var status: String?
    public var reason: String?
    public var url: String?
    /// Who picked it up, when the host recorded it; the agent waits while they have it.
    public var takenBy: String?
    public var takenAt: Int64?
    /// What the person says they did, kept with a resolved request.
    public var note: String?
}

extension TakeoverRequest {
    public var isLive: Bool { status == "waiting" || status == "taken" }
}

extension HostClient {
    /// Takes a takeover request in hand: the bot's agent waits until it is done.
    public func takeTakeover(entryId: String) async throws {
        let _: Empty = try await call("takeTakeover", ["entryId": entryId])
    }

    /// Says the step is done, with what was done: the agent goes on from the note.
    public func resolveTakeover(entryId: String, note: String?) async throws {
        struct Body: Encodable { var entryId: String; var note: String? }
        let _: Empty = try await call("resolveTakeover", Body(entryId: entryId, note: note))
    }
}
