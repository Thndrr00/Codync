import CodyncKit
import SwiftUI

/// The server's Library: every bot's artifacts and files of one computer, newest first, by day. It opens from the
/// roster's Library button; each row does what it does in a chat.
public struct ServerLibraryView: View {
    let stores: [BotStore]
    @State private var computer: ComputerID?
    @State private var filter: ServerLibraryFilter = .all
    @State private var items: [ServerLibraryItem] = []
    @State private var loaded = false
    @State private var error: String?
    @State private var viewing: Viewing?
    @State private var artifact: ArtifactTarget?

    public init(stores: [BotStore]) {
        self.stores = stores
    }

    private var store: BotStore? {
        stores.first { $0.computer.id == computer } ?? stores.first
    }

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Server Library")
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    if stores.count > 1 {
                        ChoicePicker(selection: Binding(get: { store?.computer.id ?? "" }, set: { computer = $0 }),
                                     options: stores.map { ($0.computer.id, $0.hostName) })
                    }
                    ChoicePicker(selection: $filter, options: ServerLibraryFilter.allCases.map { ($0, $0.title) })
                    if let error {
                        Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                    }
                    content
                }
                .padding(20)
            }
        }
        .background(Palette.background)
        .tracked(.screenViewed(.library))
        .task(id: store?.computer.id ?? "") { await load() }
        .codyncSheet(item: $viewing) { shown in
            if let store {
                switch shown {
                case .attachment(let file, let botId):
                    AttachmentViewer(file: file, botId: botId).environment(store)
                case .shared(let file, let entryId, _):
                    AttachmentViewer(shared: file, entryId: entryId).environment(store)
                }
            }
        }
        .codyncSheet(item: $artifact) { target in
            if let store { ArtifactView(target: target).environment(store) }
        }
        .fileExportSheetForServer(store)
    }

    @ViewBuilder private var content: some View {
        if let store {
            let shown = filter.apply(to: items)
            if !shown.isEmpty {
                ForEach(ServerLibraryItem.sections(shown), id: \.period) { section in
                    CardSection(section.period.rawValue) {
                        ForEach(section.items) { item in row(item, store: store) }
                    }
                }
            } else if loaded {
                empty
            } else {
                Spinner(size: 20).frame(maxWidth: .infinity).padding(.top, 40)
            }
        } else {
            empty
        }
    }

    @ViewBuilder private func row(_ item: ServerLibraryItem, store: BotStore) -> some View {
        let line = detail(item)
        switch item.item.kind {
        case .artifact(let artifact):
            Button {
                self.artifact = ArtifactTarget(botId: artifact.botId, id: artifact.id, title: artifact.title)
            } label: {
                LibraryLine(title: artifact.title, detail: line) { symbol(ArtifactCard.symbol) }
            }
            .buttonStyle(.plain)
        case .file(let file, let entryId) where item.image:
            Button { viewing = .shared(file, entryId: entryId, botId: item.botId) } label: {
                LibraryLine(title: file.name, detail: line) {
                    AttachmentImage(shared: file, entryId: entryId, maxPixels: 120)
                        .frame(width: 36, height: 36)
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
            }
            .buttonStyle(.plain)
        case .file(let file, let entryId):
            Button { download(file, entryId: entryId, botId: item.botId, store: store) } label: {
                LibraryLine(title: file.name, detail: "\(line) · \(FileDownloadCard.detail(store.fileDownloads.states[file.id], size: file.size))") {
                    symbol(AttachmentIcon.symbol(file.name))
                }
            }
            .buttonStyle(.plain)
        case .attachment(let file, _) where item.image:
            Button { viewing = .attachment(file, botId: item.botId) } label: {
                LibraryLine(title: file.name, detail: line) {
                    AttachmentImage(file: file, botId: item.botId, maxPixels: 120, contentMode: .fill)
                        .frame(width: 36, height: 36)
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
            }
            .buttonStyle(.plain)
        case .attachment(let file, _):
            // Like its chat card, nothing to open on the phone: the agent reads it on the computer.
            LibraryLine(title: file.name, detail: line) { symbol(AttachmentIcon.symbol(file.name)) }
        }
    }

    /// "From {bot}": whose bot published or shared it.
    private func detail(_ item: ServerLibraryItem) -> String {
        L10n.tr("From {name}").replacingOccurrences(of: "{name}", with: item.botName)
    }

    private func symbol(_ name: String) -> some View {
        Image(systemName: name).font(.system(size: 18)).foregroundStyle(Palette.secondary)
    }

    @ViewBuilder private var empty: some View {
        VStack(spacing: 8) {
            Image(systemName: "books.vertical").font(.system(size: 34)).foregroundStyle(Palette.tertiary)
            Text(L10n.tr("Library is empty")).font(.headline).foregroundStyle(Palette.text)
            Text(L10n.tr("Artifacts and files from every bot on this computer show here."))
                .font(.subheadline)
                .foregroundStyle(Palette.secondary)
                .multilineTextAlignment(.center)
        }
        .multilineTextAlignment(.center)
        .padding(.vertical, 32)
        .frame(maxWidth: .infinity)
    }

    private func load() async {
        guard let store, store.hasServerLibrary else {
            loaded = true
            return
        }
        items = []
        loaded = false
        do {
            items = try await store.loadServerLibrary()
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
        loaded = true
    }

    private func download(_ file: SharedFile, entryId: String, botId: String, store: BotStore) {
        Motion.animate {
            if case .downloading = store.fileDownloads.states[file.id] {
                store.fileDownloads.cancel(file.id)
            } else {
                store.fileDownloads.save(client: store.client, entryId: entryId, botId: botId, file: file)
            }
        }
    }
}

/// A picture to show full size: one the user sent, or one a bot shared.
private enum Viewing: Identifiable {
    case attachment(Attachment, botId: String)
    case shared(SharedFile, entryId: String, botId: String)

    var id: String {
        switch self {
        case .attachment(let file, let botId): "attachment/\(botId)/\(file.id)"
        case .shared(let file, let entryId, let botId): "file/\(botId)/\(entryId)/\(file.id)"
        }
    }
}

/// One row: its picture (a symbol or a thumbnail), the name, and whose bot it is under it.
private struct LibraryLine<Leading: View>: View {
    let title: String
    let detail: String
    @ViewBuilder let leading: Leading

    var body: some View {
        HStack(spacing: 12) {
            leading.frame(width: 36, height: 36)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).foregroundStyle(Palette.text).lineLimit(2)
                Text(detail).font(.caption).foregroundStyle(Palette.secondary).lineLimit(2)
            }
            Spacer(minLength: 0)
        }
        .contentShape(Rectangle())
    }
}

private extension View {
    /// The system share sheet for a file downloaded from any bot of this computer.
    func fileExportSheetForServer(_ store: BotStore?) -> some View {
        codyncSheet(item: Binding<FileDownloads.Export?>(
            get: { store?.fileDownloads.export },
            set: { if $0 == nil { store?.fileDownloads.dismissExport() } }
        )) { item in
            FileExportSheet(url: item.url) { store?.fileDownloads.dismissExport() }
        }
    }
}
