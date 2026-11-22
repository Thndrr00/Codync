import AVKit
import CodyncKit
import SwiftUI

/// A clip in a message (sent, or made by a bot), played in place; its bytes come from the computer
/// like a picture's, written to a temporary file the player can open.
/// Ported from OpenBot 0.70.0 `WorkflowImageNode.tsx`.
struct AttachmentVideo: View {
    let name: String
    private let id: String
    /// Set for a clip a bot shared, which can be downloaded from the player itself.
    private let shared: (entryId: String, botId: String, file: SharedFile)?
    private let load: @MainActor (BotStore) async -> Data?
    @Environment(BotStore.self) private var model
    @State private var url: URL?
    /// Opens the clip full size, where pinch and double-tap zoom apply like a picture's.
    @State private var fullscreen = false

    init(shared file: SharedFile, entryId: String, botId: String) {
        name = file.name
        id = file.id
        shared = (entryId, botId, file)
        load = { await $0.sharedFileData(file, entryId: entryId) }
    }

    init(file: Attachment, botId: String) {
        name = file.name
        id = file.id
        shared = nil
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
        .frame(maxWidth: .infinity, maxHeight: 320)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(alignment: .topTrailing) {
            if url != nil {
                HStack(spacing: 6) {
                    // Download is a control on the player, not only in a long-press menu (#11094).
                    if let shared {
                        let downloading: Bool = {
                            if case .downloading = model.fileDownloads.states[id] { return true }
                            return false
                        }()
                        Button {
                            Motion.animate {
                                if downloading { model.fileDownloads.cancel(id) }
                                else { model.fileDownloads.save(client: model.client, entryId: shared.entryId, botId: shared.botId, file: shared.file) }
                            }
                        } label: {
                            Image(systemName: downloading ? "xmark" : "arrow.down.to.line")
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(.white)
                                .padding(7)
                                .background(.black.opacity(0.6), in: Circle())
                        }
                        .buttonStyle(PressScale())
                        .accessibilityLabel(downloading ? L10n.tr("Cancel download") : L10n.tr("Download"))
                    }
                    Button { fullscreen = true } label: {
                        Image(systemName: "arrow.up.left.and.arrow.down.right")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(.white)
                            .padding(7)
                            .background(.black.opacity(0.6), in: Circle())
                    }
                    .buttonStyle(PressScale())
                    .accessibilityLabel(L10n.tr("Full screen"))
                }
                .padding(6)
            }
        }
        .accessibilityLabel(name)
        .codyncSheet(isPresented: $fullscreen) {
            VStack(spacing: 0) {
                ModalHeader(name)
                if let url {
                    MediaZoom {
                        VideoPlayer(player: AVPlayer(url: url))
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                            .padding(12)
                    }
                }
            }
            .background(Palette.background)
        }
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
