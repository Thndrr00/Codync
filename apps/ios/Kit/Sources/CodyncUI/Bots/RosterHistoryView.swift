import CodyncKit
import SwiftUI

public struct RosterHistoryView: View {
    let stores: [BotStore]
    let open: (BotStore, RosterSearchHit) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var browser = RosterBrowser()
    @State private var opening: Task<Void, Never>?

    public init(stores: [BotStore], open: @escaping (BotStore, RosterSearchHit) -> Void = { _, _ in }) {
        self.stores = stores
        self.open = open
    }

    public var body: some View {
        VStack(spacing: 12) {
            ModalHeader(browser.selected == nil ? "Search history" : "Search context")
            if let selected = browser.selected, let entries = browser.context {
                Button("Back to results") { browser.closeContext() }.buttonStyle(.secondary)
                Button(selected.hit.threadId == nil ? "Continue in chat" : "Continue in thread") {
                    selected.store.continueRosterHit(selected.hit)
                    open(selected.store, selected.hit)
                    dismiss()
                }.buttonStyle(.secondary)
                Text(selected.hit.threadId == nil ? "Main chat" : "Thread").font(.caption).foregroundStyle(Palette.secondary)
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 16) {
                            ForEach(entries.filter { $0.threadId == selected.hit.threadId && ($0.kind == "user" || $0.kind == "agent" && $0.data.final == true) }) { entry in
                                VStack(alignment: .leading, spacing: 6) {
                                    Text(entry.id == selected.hit.id ? "Matching message" : entry.kind == "user" ? "You" : "Agent")
                                        .font(.caption).foregroundStyle(Palette.secondary)
                                    Text(entry.data.text ?? "").textSelection(.enabled)
                                }
                                .id(entry.id)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(12).background(Palette.surface, in: RoundedRectangle(cornerRadius: 12))
                            }
                        }.padding(16)
                    }
                    .onAppear { proxy.scrollTo(selected.hit.id, anchor: .center) }
                }
            } else {
                SearchField(text: $query).padding(.horizontal, 16)
                if let error = browser.error { Text(error).foregroundStyle(Palette.danger).padding(.horizontal, 16) }
                if browser.loading { Text("Searching…").foregroundStyle(Palette.secondary) }
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        if !browser.loading && browser.error == nil {
                            Text(query.isEmpty ? "Search messages, including threads." : browser.matches.isEmpty ? "No matching messages" : "")
                                .foregroundStyle(Palette.secondary)
                        }
                        ForEach(browser.matches) { match in
                            Button {
                                opening?.cancel()
                                opening = Task { await browser.open(match) }
                            } label: {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text("\(match.store.bots[match.hit.botId]?.name ?? "Bot") · \(match.hit.threadId == nil ? "Main chat" : "Thread")").font(.subheadline.weight(.semibold))
                                    Text(match.hit.text).font(.subheadline).foregroundStyle(Palette.secondary).lineLimit(3)
                                }.frame(maxWidth: .infinity, alignment: .leading)
                            }
                            .buttonStyle(.secondary).disabled(browser.loading)
                        }
                        if browser.canLoadMore {
                            Button("More messages") { opening = Task { await browser.search(query, stores: stores, more: true) } }
                                .buttonStyle(.secondary).disabled(browser.loading)
                        }
                    }.padding(16)
                }
            }
        }
        .task(id: query) { opening?.cancel(); await browser.search(query, stores: stores) }
        .onDisappear { opening?.cancel() }
    }
}

