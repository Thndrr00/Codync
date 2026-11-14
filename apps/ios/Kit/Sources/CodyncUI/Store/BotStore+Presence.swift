import CodyncKit
import Foundation

extension BotStore {
    /// Says this device is looking at the chat, typing or not, and keeps who else is there (`presence`).
    /// Presence is decoration: a lost look just means the next one.
    func look(botId: String, typing: Bool? = nil) {
        Task { [weak self] in
            guard let self, let people = try? await withLink({ try await $0.presence(botId: botId, typing: typing) }) else { return }
            self.people[botId] = people
        }
    }

    /// Whether this device is typing in the chat (the others are told, at most once a second).
    func isTyping(_ botId: String) -> Bool {
        (typingUntil[botId] ?? .distantPast) > .now
    }

    /// A key went into the chat's box: the others see the typing for the next three seconds.
    func noteTyping(_ botId: String) {
        let now = Date.now
        typingUntil[botId] = now.addingTimeInterval(3)
        if let told = typingTold[botId], now.timeIntervalSince(told) < 1 { return }
        typingTold[botId] = now
        look(botId: botId, typing: true)
    }

    /// The chat is left or the message went: nothing here counts as typing any more.
    func stopTyping(_ botId: String) {
        typingUntil[botId] = nil
        typingTold[botId] = nil
    }
}
