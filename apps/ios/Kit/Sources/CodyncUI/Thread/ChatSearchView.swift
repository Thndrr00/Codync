import CodyncKit
import SwiftUI

/// The device's "Animate search results" choice (OpenBot 004). Settings writes the same key.
public enum SearchMotionStyle: String, CaseIterable, Sendable {
    case blur, fade, none

    public static let title = "Animate search results"
    public static let storageKey = "codync.animateSearchResults"

    public var label: String {
        switch self {
        case .blur: "Blur"
        case .fade: "Fade"
        case .none: "None"
        }
    }
}

/// What a search row animates with after the device setting and Reduce Motion meet.
public enum SearchMotion: Sendable, Equatable { case fade, blur, none }

/// Pure: Reduce Motion and "None" switch animation off.
public func searchMotion(_ style: SearchMotionStyle, reduce: Bool) -> SearchMotion {
    if reduce || style == .none { return .none }
    return style == .blur ? .blur : .fade
}

/// Pure: the three rows Settings shows for the device's search animation, in their stable order.
public func searchMotionOptions() -> [(id: SearchMotionStyle, label: String)] {
    SearchMotionStyle.allCases.map { (id: $0, label: $0.label) }
}

/// Pure: the Searching spinner appears only after the host has had `delayMs` to answer.
public func showSpinner(startedAt: Date?, now: Date = .now, delayMs: TimeInterval = 0.2) -> Bool {
    guard let startedAt else { return false }
    return now.timeIntervalSince(startedAt) >= delayMs
}

/// Pure: prior hits stay while an edited query loads; a cleared field's first word starts empty.
public func hitsAfter(queryChanged: Bool, cleared: Bool) -> Bool { !(queryChanged && cleared) }

/// Search one chat's messages with date, author and "has file" filters. Every result reuses the
/// host's chat-history search and the same context rows the roster uses to jump to a message.
struct ChatSearchView: View {
    let botId: String
    /// Opens a hit's thread when it lives there; a main-chat hit stays in the context sheet.
    let openThread: (String) -> Void
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(SearchMotionStyle.storageKey) private var motionStyle: SearchMotionStyle = .fade
    @State private var search: ChatSearchModel
    @State private var opening: Task<Void, Never>?

    private var motion: SearchMotion { searchMotion(motionStyle, reduce: reduceMotion) }

    init(botId: String, openThread: @escaping (String) -> Void) {
        self.botId = botId
        self.openThread = openThread
        _search = State(initialValue: ChatSearchModel(botId: botId))
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(search.context == nil ? "Search messages" : "Message context")
            if let selected = search.selected, let entries = search.context {
                contextView(selected: selected, entries: entries)
            } else {
                resultsView
            }
        }
        .background(Palette.background)
        .task(id: search.filterKey) { await search.search(store: model) }
        .onDisappear { opening?.cancel() }
    }

    private var resultsView: some View {
        VStack(spacing: 8) {
            SearchField("Search messages", text: $search.query)
                .padding(.horizontal, 16)
            VStack(spacing: 8) {
                HStack(spacing: 10) {
                    ChoicePicker(selection: $search.range, options: ChatSearchRange.allCases.map { (id: $0, label: $0.title) })
                    Spacer(minLength: 0)
                    Toggle(isOn: $search.hasFile) { Text(L10n.tr("Has file")) }
                        .toggleStyle(.codync)
                        .font(.subheadline)
                        .accessibilityLabel(L10n.tr("Has file"))
                }
                SearchField("Author", text: $search.author)
            }
            .padding(.horizontal, 16)
            if let error = search.error {
                Text(error).font(.caption).foregroundStyle(Palette.danger).frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 16)
            }
            if search.spinnerVisible { Text(L10n.tr("Searching…")).font(.caption).foregroundStyle(Palette.secondary) }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    if !search.loading && search.error == nil && search.hits.isEmpty {
                        Text(search.query.isEmpty ? L10n.tr("Search this conversation.")
                             : L10n.tr("No matching messages."))
                            .font(.body).foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    ForEach(search.hits) { hit in
                        Button { open(hit) } label: { SearchHitRow(hit: hit, query: search.query) }
                            .buttonStyle(.secondary)
                            .disabled(search.loading)
                            .transition(resultTransition)
                    }
                }
                .padding(16)
                .animation(Motion.reduced(Motion.layout, reduceMotion), value: search.hits.map(\.id))
            }
        }
    }

    /// Rows fade in and out, or rise out of an iOS-only blur veil, on the configured style.
    private var resultTransition: AnyTransition {
        switch motion {
        case .none: .identity
        case .fade: .opacity
        case .blur: .modifier(active: SearchBlurVeil(active: true), identity: SearchBlurVeil(active: false))
        }
    }

    /// The matched message sits highlighted among its neighbours: the same context the roster shows,
    /// with a way to continue in the chat or its thread.
    private func contextView(selected: ChatSearchModel.Hit, entries: [Entry]) -> some View {
        VStack(spacing: 8) {
            HStack(spacing: 10) {
                Button(L10n.tr("Back to results")) { search.closeContext() }.buttonStyle(.secondary)
                Button(selected.threadId == nil ? L10n.tr("Continue in chat") : L10n.tr("Continue in thread")) {
                    continueTo(selected)
                }
                .buttonStyle(.primary)
            }
            .padding(.horizontal, 16)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        ForEach(entries) { entry in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(entry.id == selected.id ? L10n.tr("Matching message")
                                     : entry.kind == "user" ? L10n.tr("You") : L10n.tr("Agent"))
                                    .font(.caption).foregroundStyle(Palette.secondary)
                                Text(MessagePreview.text(entry.data.text ?? "")).textSelection(.enabled)
                            }
                            .id(entry.id)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(12)
                            .background(entry.id == selected.id ? Palette.accent.opacity(0.12) : Palette.surface,
                                        in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                        }
                    }
                    .padding(16)
                }
                .onAppear {
                    withAnimation(Motion.reduced(Motion.layout, reduceMotion)) {
                        proxy.scrollTo(selected.id, anchor: .center)
                    }
                }
            }
        }
    }

    private func open(_ hit: ChatSearchModel.Hit) {
        opening?.cancel()
        opening = Task { await search.open(hit, store: model) }
    }

    /// Closes this sheet first: two sheets can't replace each other, so a thread opens afterwards.
    private func continueTo(_ hit: ChatSearchModel.Hit) {
        dismiss()
        guard let threadId = hit.threadId else { return }
        Task {
            try? await Task.sleep(for: .milliseconds(250))
            openThread(threadId)
        }
    }
}

/// The blur veil is the one iOS-only part of OpenBot 004's search motion (Android just fades).
private struct SearchBlurVeil: ViewModifier {
    let active: Bool

    func body(content: Content) -> some View {
        content.opacity(active ? 0 : 1).blur(radius: active ? 6 : 0)
    }
}

/// A search result: the host's snippet with the query tinted, when it was written and where it lives.
private struct SearchHitRow: View {
    let hit: ChatSearchModel.Hit
    let query: String

    private var location: String { hit.threadId == nil ? L10n.tr("Main chat") : L10n.tr("Thread") }
    private var at: String { RelativeTime.day(Date(milliseconds: hit.createdAt)) }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(highlight(hit.text, query)).font(.subheadline).foregroundStyle(Palette.text).lineLimit(3)
            HStack(spacing: 6) {
                Text(location)
                Text("·")
                Text(at)
            }
            .font(.caption).foregroundStyle(Palette.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(location), \(at), \(MessagePreview.text(hit.text))")
    }

    private func highlight(_ text: String, _ query: String) -> AttributedString {
        var out = AttributedString(MessagePreview.text(text))
        let needle = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !needle.isEmpty, var range = out.range(of: needle, options: .caseInsensitive) else { return out }
        out[range].foregroundColor = Palette.accent
        out[range].font = .body.bold()
        while let next = out.range(of: needle, options: .caseInsensitive, range: range.upperBound..<out.endIndex) {
            out[next].foregroundColor = Palette.accent
            out[next].font = .body.bold()
            range = next
        }
        return out
    }
}
