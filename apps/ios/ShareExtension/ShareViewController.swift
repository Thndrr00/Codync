import UIKit
import UniformTypeIdentifiers

/// Receives text or a link from another app's share sheet and leaves it for Codync, which turns it
/// into a draft in the bot the person picks. Nothing is sent from here; the app does the rest.
final class ShareViewController: UIViewController {
    private var group: String {
        Bundle.main.object(forInfoDictionaryKey: "CodyncAppGroup") as? String ?? "group.com.pokai.Codync"
    }

    private var appScheme: String {
        Bundle.main.object(forInfoDictionaryKey: "CodyncURLScheme") as? String ?? "codync"
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        Task {
            let text = await collectedText()
            if !text.isEmpty { write(text) }
            finish()
        }
    }

    private func collectedText() async -> String {
        guard let items = extensionContext?.inputItems as? [NSExtensionItem] else { return "" }
        var parts: [String] = []
        for item in items {
            if let subject = item.attributedContentText?.string.trimmingCharacters(in: .whitespacesAndNewlines),
               !subject.isEmpty, !parts.contains(subject) {
                parts.append(subject)
            }
            for provider in item.attachments ?? [] {
                if provider.hasItemConformingToTypeIdentifier(UTType.url.identifier),
                   let url = await loadURL(provider) {
                    parts.append(url.absoluteString)
                } else if provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier),
                          let text = await loadText(provider) {
                    parts.append(text.trimmingCharacters(in: .whitespacesAndNewlines))
                }
            }
        }
        return String(parts.filter { !$0.isEmpty }.joined(separator: "\n\n").prefix(20_000))
    }

    private func loadText(_ provider: NSItemProvider) async -> String? {
        await withCheckedContinuation { continuation in
            provider.loadItem(forTypeIdentifier: UTType.plainText.identifier, options: nil) { value, _ in
                continuation.resume(returning: value as? String)
            }
        }
    }

    private func loadURL(_ provider: NSItemProvider) async -> URL? {
        await withCheckedContinuation { continuation in
            provider.loadItem(forTypeIdentifier: UTType.url.identifier, options: nil) { value, _ in
                continuation.resume(returning: value as? URL)
            }
        }
    }

    private func write(_ text: String) {
        guard let directory = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)
        else { return }
        let file = directory.appendingPathComponent("share-intake.txt")
        try? text.write(to: file, atomically: true, encoding: .utf8)
        UserDefaults(suiteName: group)?.set(true, forKey: "share-intake-pending")
    }

    private func finish() {
        // Bring Codync forward when the system allows it; either way it reads the intake on next active.
        if let url = URL(string: "\(appScheme)://share") {
            var responder: UIResponder? = self
            while let current = responder {
                if current.responds(to: Selector(("openURL:"))) {
                    current.perform(Selector(("openURL:")), with: url)
                    break
                }
                responder = current.next
            }
        }
        extensionContext?.completeRequest(returningItems: nil)
    }
}
