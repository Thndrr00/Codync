import CodyncKit
import Foundation

extension BotStore {
    /// Pins or unpins a message; the pin shows at once, then the computer's copy replaces it.
    public func setPinned(_ entry: Entry) {
        let pinned = entry.data.pinned == nil
        if let i = entries[entry.botId]?.firstIndex(where: { $0.id == entry.id }) {
            entries[entry.botId]?[i].data.pinned = pinned
                ? Pinned(by: Pinner(kind: "person"), at: Int64(Date.now.timeIntervalSince1970 * 1000))
                : nil
        }
        Task { [weak self] in
            guard let self else { return }
            do {
                try await withLink { [weak self] client in
                    self?.upsert(try await client.setPinned(botId: entry.botId, messageId: entry.id, pinned: pinned))
                }
            } catch {
                lastError = error.localizedDescription
            }
        }
    }

    /// Loads the chat's pins, newest first. A failure keeps what was shown.
    public func refreshPins(_ botId: String) {
        Task { [weak self] in
            guard let self else { return }
            guard let pins = try? await withLink({ try await $0.pinnedMessages(botId: botId) }) else { return }
            self.pins[botId] = pins
        }
    }

    /// Takes a pin back from the list, where the message itself may not be loaded.
    public func unpin(_ pin: PinnedMessage, botId: String) {
        pins[botId]?.removeAll { $0.messageId == pin.messageId }
        if let entry = entries[botId]?.first(where: { $0.id == pin.messageId }) {
            setPinned(entry)
            return
        }
        Task { [weak self] in
            guard let self else { return }
            do { _ = try await withLink { try await $0.setPinned(botId: botId, messageId: pin.messageId, pinned: false) } }
            catch { lastError = error.localizedDescription }
        }
    }
}
