import CodyncKit
import CodyncUI
import Foundation

/// One file shared from another app, read before the app group's copy is removed.
struct SharedIntakeFile {
    let name: String
    let data: Data
}

/// Text, a link or files shared from another app, waiting to become a message.
struct SharedIntake: Identifiable {
    let id = UUID()
    let text: String
    let files: [SharedIntakeFile]
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
        defer {
            try? FileManager.default.removeItem(at: directory.appendingPathComponent("share-intake", isDirectory: true))
            try? FileManager.default.removeItem(at: directory.appendingPathComponent("share-intake.json"))
            try? FileManager.default.removeItem(at: directory.appendingPathComponent("share-intake.txt"))
        }
        let inbox = directory.appendingPathComponent("share-intake", isDirectory: true)
        let metadataURL = directory.appendingPathComponent("share-intake.json")
        var text = ""
        var names: [String] = []
        if let data = try? Data(contentsOf: metadataURL),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            text = json["text"] as? String ?? ""
            names = json["files"] as? [String] ?? []
        } else if let legacy = try? String(contentsOf: directory.appendingPathComponent("share-intake.txt"), encoding: .utf8) {
            text = legacy
        }
        let files: [SharedIntakeFile] = names.compactMap { name in
            let url = inbox.appendingPathComponent(name)
            guard let data = try? Data(contentsOf: url), data.count <= 100 * 1024 * 1024 else { return nil }
            // The first segment is the extension's index (kept so duplicates keep their order).
            let shown = name.split(separator: "-", maxSplits: 1).dropFirst().joined(separator: "-")
            return SharedIntakeFile(name: shown.isEmpty ? name : shown, data: data)
        }.prefix(10).map { $0 }
        let trimmed = String(text.trimmingCharacters(in: .whitespacesAndNewlines).prefix(20_000))
        guard !trimmed.isEmpty || !files.isEmpty else { return nil }
        return SharedIntake(text: trimmed, files: files)
    }
}
