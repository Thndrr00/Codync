import CodyncUI
import Foundation
import SwiftUI

/// RSS and Atom feeds this computer follows, and the newest headlines (iOS `NewsFeedsSection`).
struct WorkspaceNewsFeedsSection: View {
    let store: BotStore
    @Environment(\.openURL) private var openURL
    @State private var feeds: [NewsFeed] = []
    @State private var items: [NewsItem] = []
    @State private var name = ""
    @State private var url = ""
    @State private var only = ""
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        CardSection("News feeds", footer: "Paste an RSS or Atom address. Codync reads it at most every ten minutes and never writes to the site; bots read the same items with news_feed.") {
            TextField("Name (optional)", text: $name).plainTextInput()
            TextField("https://example.com/feed.xml", text: $url)
                .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput()
            HStack {
                Spacer()
                Button("Add") { add() }
                    .buttonStyle(.secondary)
                    .disabled(url.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || busy)
            }

            ForEach(feeds) { feed in
                HStack(spacing: 8) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(feed.name).foregroundStyle(Palette.text)
                        Text(feed.url).font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
                    }
                    Spacer(minLength: 0)
                    Button(only == feed.name ? "All feeds" : "Only this") { only = only == feed.name ? "" : feed.name }
                        .buttonStyle(.secondary)
                    Button("Remove") { remove(feed) }
                        .buttonStyle(.plain)
                        .font(.caption)
                        .foregroundStyle(Palette.danger)
                }
            }
            if feeds.isEmpty, !busy {
                Text("No news feeds yet.").font(.caption).foregroundStyle(Palette.secondary)
            }

            Text(only.isEmpty ? "Newest items" : only)
                .font(.caption).fontWeight(.semibold).foregroundStyle(Palette.secondary)
            let visible = only.isEmpty ? items : items.filter { $0.feed == only }
            ForEach(visible) { item in
                VStack(alignment: .leading, spacing: 2) {
                    Button {
                        if let link = item.url, let destination = URL(string: link) { openURL(destination) }
                    } label: {
                        Text(item.title).foregroundStyle(Palette.text).multilineTextAlignment(.leading)
                    }
                    .buttonStyle(.plain)
                    Text([item.feed, when(item.published)].compactMap { $0 }.joined(separator: " · "))
                        .font(.caption)
                        .foregroundStyle(Palette.secondary)
                    if !item.summary.isEmpty {
                        Text(item.summary).font(.caption).foregroundStyle(Palette.tertiary).lineLimit(3)
                    }
                }
            }
            if visible.isEmpty, !busy {
                Text("No items yet. Add a feed or refresh.").font(.caption).foregroundStyle(Palette.secondary)
            }
            HStack {
                Spacer()
                Button(busy ? "Reading…" : "Refresh") { load() }
                    .buttonStyle(.secondary)
                    .disabled(busy)
            }
            if let error {
                Text(error).font(.caption).foregroundStyle(Palette.danger)
            }
        }
        .task { load() }
    }

    private func load() {
        busy = true
        Task {
            do {
                let (list, news) = try await (store.newsFeeds(), store.newsItems())
                feeds = list
                items = news
                error = nil
            } catch { self.error = message(error) }
            busy = false
        }
    }

    private func add() {
        busy = true
        Task {
            do {
                _ = try await store.addNewsFeed(
                    name: name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "Feed" : name,
                    url: url.trimmingCharacters(in: .whitespacesAndNewlines)
                )
                name = ""
                url = ""
                let (list, news) = try await (store.newsFeeds(), store.newsItems())
                feeds = list
                items = news
                error = nil
            } catch { self.error = message(error) }
            busy = false
        }
    }

    private func remove(_ feed: NewsFeed) {
        Task {
            do {
                _ = try await store.removeNewsFeed(feed.id)
                if only == feed.name { only = "" }
                let (list, news) = try await (store.newsFeeds(), store.newsItems())
                feeds = list
                items = news
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func when(_ ms: Int?) -> String? {
        guard let ms else { return nil }
        return Date(timeIntervalSince1970: Double(ms) / 1000)
            .formatted(.dateTime.day().month(.abbreviated))
    }

    private func message(_ error: Error) -> String {
        (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
}
