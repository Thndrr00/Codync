import AVKit
import CodyncKit
import SwiftUI

/// A clip in a message (sent, or made by a bot), played in place; its bytes come from the computer
/// like a picture's, written to a temporary file the player can open.
/// Ported from OpenBot 0.70.0 `WorkflowImageNode.tsx`.
struct AttachmentVideo: View {
    let name: String
    private let id: String
    private let load: @MainActor (BotStore) async -> Data?
    @Environment(BotStore.self) private var model
    @State private var url: URL?

    init(shared file: SharedFile, entryId: String) {
        name = file.name
        id = file.id
        load = { await $0.sharedFileData(file, entryId: entryId) }
    }

    init(file: Attachment, botId: String) {
        name = file.name
        id = file.id
        load = { await $0.attachmentData(file, bot: botId) }
    }

    var body: some View {
        Group {
            if let url {
                VideoPlayer(player: AVPlayer(url: url))
            } else {
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(.black.opacity(0.75))
                    .frame(height: 190)
                    .overlay { Image(systemName: "film").foregroundStyle(Palette.secondary) }
            }
        }
        .frame(maxWidth: 340, maxHeight: 280)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .accessibilityLabel(name)
        // Asked again when the computer comes online: a chat shown from the cache before it connects
        // gets its clips once it does. A loaded clip is never fetched again.
        .task(id: [id, model.connection == .online ? "online" : "waiting"]) {
            guard url == nil, let data = await load(model) else { return }
            let named = name
            url = await Task.detached { Self.written(data, named: named) }.value
        }
        .onDisappear {
            guard let url else { return }
            try? FileManager.default.removeItem(at: url)
            self.url = nil
        }
    }

    /// The clip's bytes in a temporary file the player can open, or nil when they can't be written.
    private static func written(_ data: Data, named name: String) -> URL? {
        let ext = (name as NSString).pathExtension
        let folder = URL.temporaryDirectory.appending(path: "codync-clips")
        let target = folder.appending(path: UUID().uuidString + (ext.isEmpty ? "" : ".\(ext)"))
        do {
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try data.write(to: target)
            return target
        } catch {
            return nil
        }
    }
}
