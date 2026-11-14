import Foundation

public extension HostClient {
    /// What this bot's agent can make, for the `/image` and `/video` picker.
    func mediaModels(botId: String) async throws -> MediaAnswer {
        try await call("mediaModels", ["botId": botId])
    }
}
