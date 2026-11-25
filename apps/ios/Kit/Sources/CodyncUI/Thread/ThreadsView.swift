import CodyncKit
import SwiftUI

/// Branches of one source message sit together, under a row that names the message.
struct ThreadGroup: Sendable {
    let source: String?
    let sourceText: String?
    let children: [ThreadInfo]
}

/// Pure: group branches that share a source; a source with one branch stays a flat row.
func nestThreads(_ threads: [ThreadInfo]) -> [ThreadGroup] {
    let branched = Dictionary(grouping: threads.filter { $0.fromEntry != nil }) { $0.fromEntry ?? "" }
    var placed = Set<String>()
    var groups: [ThreadGroup] = []
    for thread in threads {
        guard let source = thread.fromEntry else {
            groups.append(ThreadGroup(source: nil, sourceText: nil, children: [thread]))
            continue
        }
        if placed.contains(source) || (branched[source]?.count ?? 0) < 2 {
            groups.append(ThreadGroup(source: nil, sourceText: nil, children: [thread]))
        } else {
            groups.append(ThreadGroup(source: source, sourceText: thread.branchText, children: branched[source] ?? []))
        }
        placed.insert(source)
    }
    return groups
}

/// Every thread of a chat, open ones or the archive, newest first with the last line of each.
/// A thread is a side conversation: opening one puts it over the chat, and a row's menu
/// renames, archives or deletes it.
struct ThreadsView: View {
    let botId: String
    /// The thread the chat is showing now: it stays in the list while a filter is applied.
    var openThreadId: String?
    /// Tells the chat to open a thread.
    let open: (String) -> Void
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var archived = false
    @State private var query = ""
    @State private var renaming: ThreadInfo?
    @State private var draftTitle = ""
    @State private var deleting: ThreadInfo?
    @State private var loadingOlder = false

    private var allThreads: [ThreadInfo] {
        (archived ? model.archivedThreads : model.threads)[botId] ?? []
    }

    /// How many open threads wait for the owner, shown as the tab's count (OpenBot t3-prototype).
    private var waiting: Int {
        (model.threads[botId] ?? []).filter { $0.status == .waiting }.count
    }

    /// Titles and last lines are searched with the host's own case-insensitive word rule; the
    /// thread currently open is kept so the reader never loses their place while narrowing the list.
    private var threads: [ThreadInfo] {
        let words = query.split(whereSeparator: \.isWhitespace).map(String.init)
        guard !words.isEmpty else { return allThreads }
        return allThreads.filter { thread in
            if thread.id == openThreadId { return true }
            let haystack = [thread.name, thread.title, thread.lastText, thread.branchText]
                .compactMap { $0 }.joined(separator: " ").lowercased()
            return words.allSatisfy { haystack.contains($0.lowercased()) }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Threads") {
                IconButton("New thread", systemImage: "plus") {
                    Task {
                        if let root = await model.startThread(botId) { show(root) }
                    }
                }
            }
            SegmentedChoice(selection: $archived, options: [(id: false, label: "Open"), (id: true, label: "Archived")])
                .padding(.horizontal, 20)
                .padding(.vertical, 4)
            SearchField("Search threads", text: $query)
                .padding(.horizontal, 16)
                .padding(.bottom, 4)
            Text(L10n.tr("A thread keeps one side conversation apart from the chat."))
                .font(.caption)
                .foregroundStyle(Palette.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.vertical, 4)
            if !archived && waiting > 0 {
                Text(L10n.tr("\(waiting) waiting for you"))
                    .font(.caption2)
                    .foregroundStyle(Palette.accent)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 20)
                    .padding(.bottom, 4)
                    .accessibilityLabel(L10n.tr("\(waiting) open threads wait for you"))
            }
            Hairline()
            if archived && model.threadHasMore["\(botId)|true"] == true {
                Button {
                    loadingOlder = true
                    model.refreshThreads(botId, archived: true, append: true)
                    Task { try? await Task.sleep(for: .milliseconds(400)); loadingOlder = false }
                } label: {
                    Text(L10n.tr(loadingOlder ? "Loading older threads…" : "Show older threads"))
                }
                .buttonStyle(SecondaryButtonStyle())
                .disabled(loadingOlder)
                .padding(.horizontal, 20)
                .padding(.vertical, 4)
                .accessibilityLabel(L10n.tr("Show older threads"))
            }
            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(nestThreads(threads), id: \.children.first?.id) { group in
                        if let source = group.source {
                            HStack(spacing: 6) {
                                Image(systemName: "arrow.triangle.branch")
                                Text(L10n.tr("Branched from") + " "
                                     + (group.sourceText?.replacingOccurrences(of: "\n", with: "") ?? ""))
                                    .lineLimit(1)
                            }
                            .font(.caption)
                            .foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.top, 8)
                            .accessibilityLabel(L10n.tr("Branched from") + " \(source)")
                        }
                        ForEach(group.children) { thread in
                            ThreadRow(thread: thread, archived: archived,
                                      lastAuthor: authorName(thread.lastAuthor),
                                      open: { show(thread.id) },
                                      rename: { renaming = thread; draftTitle = thread.title },
                                      archive: { model.setThreadArchived(thread.id, botId: botId, archived: !archived) },
                                      delete: { deleting = thread })
                                .padding(.leading, group.source == nil ? 0 : 20)
                        }
                    }
                    if threads.isEmpty {
                        Text(emptyMessage)
                            .font(.body)
                            .foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, 12)
                    }
                }
                .padding(16)
            }
        }
        .background(Palette.background)
        .task(id: archived) { model.refreshThreads(botId, archived: archived) }
        .codyncDialog("Rename thread", isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } }),
                      field: DialogField("A short name", text: $draftTitle)) {
            guard let thread = renaming else { return [] }
            return [DialogAction("Save") { model.renameThread(thread.id, botId: botId, title: draftTitle) }]
        }
        .codyncDialog("Delete thread?", isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } }),
                      message: "Its replies go with it; the chat stays.") {
            guard let thread = deleting else { return [] }
            return [DialogAction("Delete thread", destructive: true) { model.deleteThread(thread.id, botId: botId) }]
        }
    }

    private var emptyMessage: String {
        if !query.isEmpty { return L10n.tr("No threads match your search.") }
        return archived ? L10n.tr("Nothing archived.")
            : L10n.tr("No threads yet. Open one from the menu, or branch from a message.")
    }

    private func authorName(_ author: String?) -> String? {
        guard let author else { return nil }
        return author == "user" ? L10n.tr("You") : model.bots[author]?.name
    }

    /// Closes this list, then opens the thread once it is gone: two sheets don't replace each other.
    private func show(_ root: String) {
        dismiss()
        Task {
            try? await Task.sleep(for: .milliseconds(250))
            open(root)
        }
    }
}

private struct ThreadRow: View {
    let thread: ThreadInfo
    let archived: Bool
    let lastAuthor: String?
    let open: () -> Void
    let rename: () -> Void
    let archive: () -> Void
    let delete: () -> Void

    private var subtitle: String? {
        let line = [lastAuthor, thread.lastText].compactMap { $0 }.joined(separator: ": ")
        if !line.isEmpty { return line }
        return thread.branchText.map { L10n.tr("Branched from") + " " + $0.replacingOccurrences(of: "\n", with: " ") }
    }

    var body: some View {
        Button(action: open) {
            HStack(spacing: 10) {
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(thread.name)
                            .font(.body.weight(.medium))
                            .foregroundStyle(Palette.text)
                            .lineLimit(1)
                        if thread.unread > 0 {
                            Circle().fill(Palette.accent).frame(width: 7, height: 7)
                        }
                    }
                    if let subtitle {
                        Text(subtitle)
                            .font(.caption)
                            .foregroundStyle(Palette.secondary)
                            .lineLimit(1)
                    }
                    Text(thread.status.label)
                        .font(.caption2)
                        .foregroundStyle(thread.status == .waiting ? Palette.accent : Palette.tertiary)
                }
                Spacer(minLength: 8)
                Text(RelativeTime.short(Date(milliseconds: thread.updatedAt)))
                    .font(.caption2)
                    .foregroundStyle(Palette.tertiary)
            }
            .padding(.vertical, 8)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .contextActions {
            var items = [
                MenuItem("Open", icon: "arrow.up.right", action: open),
                MenuItem("Rename", icon: "pencil", action: rename),
            ]
            items.append(archived ? MenuItem("Restore", icon: "tray.and.arrow.up", action: archive)
                                  : MenuItem("Archive", icon: "archivebox", action: archive))
            items.append(MenuItem("Delete", icon: "trash", destructive: true, divider: true, action: delete))
            return items
        }
    }
}
