import CodyncKit
import SwiftUI

/// Asks this chat what it already knows: its messages, its pages and its memory, best match first, each with its
/// snippet and date. A message opens in its context, a page or a note opens here.
public struct ChatSearchView: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var side = ChatSearchAuthor.everyone
    @State private var scope = ChatSearchScope.everything
    @State private var tools = false
    @State private var hits: [ChatSearchHit] = []
    @State private var loading = false
    @State private var error: String?
    @State private var context: (hit: ChatSearchHit, entries: [Entry])?
    @State private var page: (title: String, body: String, note: Bool)?
    @State private var search: Task<Void, Never>?
    @State private var opening: Task<Void, Never>?

    public init(botId: String) { self.botId = botId }

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let page {
                ModalHeader(page.title)
                Text(page.note ? "A note from its memory" : "A page of this bot")
                    .font(.caption).foregroundStyle(Palette.secondary).padding(.horizontal, 16)
                ScrollView {
                    MarkdownText(page.body).padding(16).frame(maxWidth: .infinity, alignment: .leading)
                }
                Button("Back to results") { self.page = nil }.buttonStyle(.secondary).padding(16)
            } else if let context {
                contextView(context.hit, entries: context.entries)
            } else {
                ModalHeader("Search this chat")
                SearchField("Words to look for", text: $query).padding(.horizontal, 16)
                VStack(alignment: .leading, spacing: 10) {
                    TabChoice(selection: $scope, options: ChatSearchScope.allCases.map { (id: $0, label: $0.title) })
                    HStack(spacing: 10) {
                        TabChoice(selection: $side, options: ChatSearchAuthor.allCases.map { (id: $0, label: $0.title) })
                        Button {
                            tools.toggle()
                        } label: {
                            Label("Tool traces", systemImage: "hammer")
                        }
                        .buttonStyle(.secondary)
                        .accessibilityAddTraits(tools ? .isSelected : [])
                    }
                }
                .padding(16)
                results
            }
        }
        .task(id: "\(query)\u{1}\(side.rawValue)\u{1}\(scope.rawValue)\u{1}\(tools)") { await run() }
        .onDisappear {
            search?.cancel()
            opening?.cancel()
        }
    }

    @ViewBuilder private var results: some View {
        if let error { Text(error).foregroundStyle(Palette.danger).padding(.horizontal, 16) }
        if loading { Text("Searching…").foregroundStyle(Palette.secondary).padding(.horizontal, 16) }
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 12) {
                if !loading, error == nil, hits.isEmpty {
                    Text(query.isEmpty
                         ? "What was said here, its pages and its memory. Every word has to appear."
                         : "Nothing here matches those words.")
                        .foregroundStyle(Palette.secondary)
                }
                ForEach(hits) { hit in
                    Button {
                        opening?.cancel()
                        opening = Task { await open(hit) }
                    } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack(spacing: 6) {
                                Image(systemName: hit.isPage ? "doc.text" : hit.isNote ? "brain" : hit.isMine ? "person" : "bubble.left")
                                Text(headline(hit)).font(.subheadline.weight(.semibold))
                                Spacer(minLength: 8)
                                Text(Date(milliseconds: hit.at).formatted(date: .abbreviated, time: .shortened))
                                    .font(.caption)
                            }
                            .foregroundStyle(Palette.secondary)
                            highlighted(hit.snippet)
                                .font(.subheadline).foregroundStyle(Palette.secondary)
                                .lineLimit(3).multilineTextAlignment(.leading)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.secondary).disabled(loading)
                }
            }.padding(16)
        }
    }

    private func contextView(_ hit: ChatSearchHit, entries: [Entry]) -> some View {
        VStack(spacing: 12) {
            ModalHeader("Search context")
            Button("Back to results") { self.context = nil }.buttonStyle(.secondary)
            if hit.threadId != nil {
                Button("Continue in thread") {
                    model.rosterThreadRequest = RosterSearchHit(id: hit.id, botId: botId, threadId: hit.threadId,
                                                               text: hit.snippet, seq: 0)
                    dismiss()
                }.buttonStyle(.secondary)
            }
            Text(hit.threadId == nil ? "Main chat" : "Thread").font(.caption).foregroundStyle(Palette.secondary)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 16) {
                        ForEach(entries.filter {
                            $0.threadId == hit.threadId && ($0.kind == "user" || $0.kind == "agent" && $0.data.final == true)
                        }) { entry in
                            VStack(alignment: .leading, spacing: 6) {
                                Text(entry.id == hit.id ? "Matching message" : entry.kind == "user" ? "You" : "Bot")
                                    .font(.caption).foregroundStyle(Palette.secondary)
                                Text(entry.data.text ?? "").textSelection(.enabled)
                            }
                            .id(entry.id)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(12).background(Palette.surface, in: RoundedRectangle(cornerRadius: 12))
                        }
                    }.padding(16)
                }
                .onAppear { proxy.scrollTo(hit.id, anchor: .center) }
            }
        }
    }

    private func headline(_ hit: ChatSearchHit) -> String {
        if hit.isPage { return "\(hit.title ?? "Page") · page" }
        if hit.isNote { return "\(hit.title ?? "Note") · \(hit.shared == true ? "shared memory" : "memory")" }
        let who = hit.isMine ? "You" : model.bots[botId]?.name ?? "Bot"
        guard let author = hit.author else { return who }
        return "\(who) · \(author)"
    }

    /// The snippet with its matches in bold: the host brackets them.
    private func highlighted(_ snippet: String) -> Text {
        snippet.split(separator: "[", omittingEmptySubsequences: false).enumerated().reduce(Text("")) { text, part in
            guard let close = part.element.firstIndex(of: "]") else { return text + Text(String(part.element)) }
            return text + Text(String(part.element[..<close])).bold() + Text(String(part.element[part.element.index(after: close)...]))
        }
    }

    private func run() async {
        search?.cancel()
        let words = query.trimmingCharacters(in: .whitespacesAndNewlines)
        error = nil
        guard !words.isEmpty else { hits = []; loading = false; return }
        search = Task {
            loading = true
            defer { loading = false }
            do {
                try await Task.sleep(for: .milliseconds(250))
                hits = try await model.searchChat(words, botId: botId, author: side, scope: scope, tools: tools)
            } catch is CancellationError {} catch { self.error = error.localizedDescription }
        }
        await search?.value
    }

    private func open(_ hit: ChatSearchHit) async {
        do {
            if hit.kind == .document {
                let page = try await model.searchPage(hit.id)
                self.page = (page.title, page.body, false)
            } else if hit.isNote {
                let body = try await model.searchNote(botId: botId, file: hit.title ?? "", shared: hit.shared == true)
                page = (hit.title ?? "", body, true)
            } else {
                let entries = try await model.chatHitContext(botId: botId, entryId: hit.id)
                context = (hit: hit, entries: entries)
            }
        } catch is CancellationError {} catch { self.error = error.localizedDescription }
    }
}
