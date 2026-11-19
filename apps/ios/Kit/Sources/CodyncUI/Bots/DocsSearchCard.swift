import CodyncKit
import Foundation
import SwiftUI

/// The `codync-docs` index a bot searches: the folder it reads, what is in it, and the two ways to change
/// it (`docsStatus` / `setDocsFolder`). Until someone names a folder, it is the bot's working folder.
struct DocsSearchCard: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var status: DocsStatus?
    @State private var error: String?
    @State private var busy = false
    @State private var pickingFolder = false
    /// Which request owns the card: a newer load or change discards the answer of an older one.
    @State private var epoch = UUID()

    var body: some View {
        CardSection("Documentation search", footer: "The bot searches this folder for documentation and code, and cites the file and line. Generated folders and what .gitignore lists are skipped.") {
            VStack(alignment: .leading, spacing: 14) {
                if let status {
                    folderRow(status)
                    ValueRow("Files") {
                        if busy { Spinner(size: 16) } else { Text(status.files.formatted()) }
                    }
                    ValueRow("Size") {
                        if busy { Spinner(size: 16) } else { Text(ByteCountFormatter.string(fromByteCount: Int64(status.bytes), countStyle: .file)) }
                    }
                    if status.truncated { truncationNote }
                } else if error == nil, !offline {
                    reading
                }
                // A failed read can be a missing working folder: choosing one is the way out, so they stay.
                actions
                if let error { failure(error, offersRetry: status == nil && !offline) }
                if offline {
                    Text(L10n.tr("Reconnect to this computer to change the indexed folder."))
                        .font(.caption)
                        .foregroundStyle(Palette.secondary)
                }
            }
        }
        .task(id: botId) { await load() }
        // The folder the host reads every time the link comes back (another session may have moved it).
        .onChange(of: model.connection) { _, connection in
            if connection == .online { Task { await load() } }
        }
        // The folder behind the index is the bot's workspace until another is chosen: following a workspace
        // change keeps the card true without a second visit.
        .onChange(of: model.bots[botId]?.cwd) { _, _ in Task { await load() } }
        .codyncSheet(isPresented: $pickingFolder) {
            FolderPicker(title: "Choose documentation folder",
                         note: "Your bot will search this folder for documentation and code.",
                         start: pickerStart) { path in
                pickingFolder = false
                Task { await change(to: path) }
            }
        }
    }

    private var offline: Bool { model.isOffline }

    /// Where the picker opens: the folder indexed now, else the bot's workspace, else the computer's home.
    private var pickerStart: String? {
        if let folder = status?.folder { return folder }
        let workspace = model.bots[botId]?.cwd ?? ""
        return workspace.isEmpty ? model.hello?.home : workspace
    }

    private func folderRow(_ status: DocsStatus) -> some View {
        Field("Folder") {            Text(status.folder)
                .font(.callout)
                .lineLimit(2)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .fieldBox()
        }
    }

    private var truncationNote: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill").font(.caption2)
            Text(L10n.tr("The index stopped at its limit of 4,000 files or 24 MB, so a search may miss some files."))
                .fixedSize(horizontal: false, vertical: true)
        }
        .font(.caption)
        .foregroundStyle(Palette.warning)
    }

    private var actions: some View {
        VStack(alignment: .leading, spacing: 10) {
            Button(L10n.tr("Choose folder")) { pickingFolder = true }
            Button(L10n.tr("Use working folder")) { Task { await change(to: "") } }
        }
        .buttonStyle(.secondary)
        .disabled(busy || offline)
    }

    private var reading: some View {
        HStack(spacing: 10) {
            Spinner(size: 16)
            Text(L10n.tr("Reading the index…")).foregroundStyle(Palette.secondary)
        }
    }

    private func failure(_ message: String, offersRetry: Bool) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(L10n.tr(message))
                .font(.footnote)
                .foregroundStyle(Palette.danger)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
            if offersRetry {
                Button(L10n.tr("Try again")) { Task { await load() } }
                    .buttonStyle(.secondary)
                    .disabled(busy || offline)
            }
        }
    }

    /// "1,234 files · 12.4 MB", laid out by the language.
    private func load() async {
        let request = UUID()
        epoch = request
        error = nil
        do {
            let fresh = try await model.docsStatus(botId: botId)
            guard request == epoch else { return }
            status = fresh
            error = nil
        } catch is CancellationError {
            return
        } catch {
            guard request == epoch else { return }
            self.error = error.localizedDescription
        }
    }

    /// Names the folder to search; an empty path goes back to the bot's working folder.
    private func change(to path: String) async {
        guard !busy, !offline else { return }
        let request = UUID()
        epoch = request
        busy = true
        error = nil
        defer { busy = false }
        do {
            let fresh = try await model.setDocsFolder(path, botId: botId)
            guard request == epoch else { return }
            status = fresh
            error = nil
        } catch {
            guard request == epoch else { return }
            self.error = error.localizedDescription
        }
    }
}
