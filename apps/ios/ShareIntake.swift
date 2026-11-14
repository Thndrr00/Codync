import CodyncKit
import Foundation

/// Text or a link shared from another app, waiting to become a message.
struct SharedIntake: Identifiable {
    let id = UUID()
    let text: String
}

/// The share sheet's inbox: the extension writes in the app group, the app takes it once.
enum ShareInbox {
    private static let pendingKey = "share-intake-pending"
    private static var defaults: UserDefaults { UserDefaults(suiteName: SharedStore.appGroup) ?? .standard }

    static func take() -> SharedIntake? {
        guard defaults.bool(forKey: pendingKey) else { return nil }
        defer { defaults.removeObject(forKey: pendingKey) }
        guard let directory = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: SharedStore.appGroup)
        else { return nil }
        let file = directory.appendingPathComponent("share-intake.txt")
        let text = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
        try? FileManager.default.removeItem(at: file)
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : SharedIntake(text: String(trimmed.prefix(20_000)))
    }
}
