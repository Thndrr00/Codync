import CodyncKit
import Foundation
import Observation

@MainActor @Observable public final class RosterPreview {
    public private(set) var entries: [Entry]?
    public private(set) var error: String?

    public init() {}

    public func load(_ botId: String, store: BotStore) async {
        entries = nil
        error = nil
        do {
            entries = try await store.rosterPreview(botId)
        } catch is CancellationError {
            // Dismissing the preview cancels its request.
        } catch {
            self.error = "Preview unavailable. Open the chat. \(error.localizedDescription)"
        }
    }
}
