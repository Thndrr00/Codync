import UIKit
import UniformTypeIdentifiers

/// Receives text, a link or a few files from another app's share sheet and leaves them for Codync,
/// which turns them into a draft in the bot the person picks. Nothing is sent from here.
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
            let collected = await collectedText()
            write(collected)
            finish()
        }
    }

    private func collectedText() async -> (text: String, files: [(name: String, url: URL)]) {
        guard let items = extensionContext?.inputItems as? [NSExtensionItem] else { return ("", []) }
        var parts: [String] = []
        var files: [(String, URL)] = []
        for item in items {
            if let subject = item.attributedContentText?.string.trimmingCharacters(in: .whitespacesAndNewlines),
               !subject.isEmpty, !parts.contains(subject) {
                parts.append(subject)
            }
            for provider in item.attachments ?? [] {
                if provider.hasItemConformingToTypeIdentifier(UTType.url.identifier),
                   let url = await loadURL(provider) {
                    parts.append(url.absoluteString)
                } else if provider.hasItemConformingToTypeIdentifier(UTType.image.identifier),
                          let file = await loadFile(provider) {
                    files.append(file)
                } else if provider.hasItemConformingToTypeIdentifier(UTType.pdf.identifier),
                          let file = await loadFile(provider) {
                    files.append(file)
                } else if provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier),
                          let text = await loadText(provider) {
                    parts.append(text.trimmingCharacters(in: .whitespacesAndNewlines))
                }
            }
        }
        let text = String(parts.filter { !$0.isEmpty }.joined(separator: "\n\n").prefix(20_000))
        return (text, Array(files.prefix(10)))
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

    private func loadFile(_ provider: NSItemProvider) async -> (name: String, url: URL)? {
        let suggested = provider.suggestedName
        return await withCheckedContinuation { continuation in
            let type = provider.registeredTypeIdentifiers.first ?? UTType.data.identifier
            provider.loadFileRepresentation(forTypeIdentifier: type) { url, _ in
                guard let url else { return continuation.resume(returning: nil) }
                let name = suggested ?? url.lastPathComponent
                continuation.resume(returning: (name, url))
            }
        }
    }

    private func write(_ collected: (text: String, files: [(name: String, url: URL)])) {
        guard let directory = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)
        else { return }
        let inbox = directory.appendingPathComponent("share-intake", isDirectory: true)
        try? FileManager.default.removeItem(at: inbox)
        try? FileManager.default.createDirectory(at: inbox, withIntermediateDirectories: true)
        var names: [String] = []
        for (index, file) in collected.files.enumerated() {
            // Each item provider's temporary file lives only for this callback: copy it now.
            let name = "\(index)-\(file.name.replacingOccurrences(of: "/", with: "-"))"
            let target = inbox.appendingPathComponent(name)
            if (try? FileManager.default.copyItem(at: file.url, to: target)) != nil { names.append(name) }
        }
        let metadata: [String: Any] = ["text": collected.text, "files": names]
        if let data = try? JSONSerialization.data(withJSONObject: metadata) {
            try? data.write(to: directory.appendingPathComponent("share-intake.json"), options: .atomic)
        }
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
