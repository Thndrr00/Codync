import CodyncKit
import Foundation

/// The New bot screen's draft, kept on this phone while the screen is left and reopened.
enum NewBotDraft {
    private static let key = "codync.newBot.draft"

    /// Pure decision: a fresh draft keeps the saved model and its reasoning option when it has none of its own.
    static func keep(saved: BotDraft?, fresh: BotDraft) -> BotDraft {
        guard let saved else { return fresh }
        var draft = fresh
        if draft.model?.isEmpty != false { draft.model = saved.model }
        var options = draft.modelOptions
        for (id, value) in saved.modelOptions where options[id]?.isEmpty != false { options[id] = value }
        draft.modelOptions = options
        return draft
    }

    static func restore() -> BotDraft {
        guard let data = UserDefaults.standard.data(forKey: key),
              let saved = try? JSONDecoder().decode(BotDraft.self, from: data) else { return BotDraft() }
        return keep(saved: saved, fresh: BotDraft())
    }

    static func save(_ draft: BotDraft) {
        guard let data = try? JSONEncoder().encode(draft) else { return }
        UserDefaults.standard.set(data, forKey: key)
    }

    static func clear() { UserDefaults.standard.removeObject(forKey: key) }
}
