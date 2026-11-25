import CodyncKit
import Foundation
import Observation

/// The date part of the chat search filter. The boundary is computed on the phone and sent to the
/// host's chat-history search as the time at or after which a hit has to have been written.
public enum ChatSearchRange: String, CaseIterable, Sendable {
    case any, today, week, month

    public var title: String {
        switch self {
        case .any: "Any time"
        case .today: "Today"
        case .week: "Past week"
        case .month: "Past month"
        }
    }

    /// Unix milliseconds at or after which a hit has to be; 0 means no lower bound.
    public func since(now: Date = .now) -> Int64 {
        let calendar = Calendar.current
        let start: Date
        switch self {
        case .any: return 0
        case .today: start = calendar.startOfDay(for: now)
        case .week: start = calendar.date(byAdding: .day, value: -7, to: now) ?? now
        case .month: start = calendar.date(byAdding: .month, value: -1, to: now) ?? now
        }
        return Int64(start.timeIntervalSince1970 * 1000)
    }
}

/// One match from the host's chat-history search. The host pages chat-visible messages newest
/// first and returns the snippet and when it was written; author and file filters read the loaded
/// context, so the search adds no wire fields.
@MainActor @Observable public final class ChatSearchModel {
    public struct Hit: Identifiable, Sendable {
        public let id: String
        public let botId: String
        public let threadId: String?
        public let text: String
        public let createdAt: Int64
    }

    public var query = ""
    public var range: ChatSearchRange = .any
    public var author = ""
    public var hasFile = false

    public private(set) var hits: [Hit] = []
    public private(set) var loading = false
    /// The spinner follows the host call, not the keystroke, so a fast answer never flashes it.
    public private(set) var spinnerVisible = false
    public private(set) var error: String?
    /// The span around the open hit, with the matching message highlighted.
    public private(set) var context: [Entry]?
    public private(set) var selected: Hit?

    private let botId: String
    private var sequence = 0
    private var lastQuery = ""
    private var spinnerTask: Task<Void, Never>?
    /// Loaded contexts by hit id, reused as filters change.
    private var details: [String: Entry] = [:]

    public init(botId: String) { self.botId = botId }

    /// A single key for the view's `.task(id:)`, so changing any filter re-runs the search.
    public var filterKey: String {
        "\(query)|\(range.rawValue)|\(author)|\(hasFile)"
    }

    /// Reuses the same host search the roster uses, with a local response that also carries the
    /// timestamp the host returns. The query and `since` are the host channel-history filters;
    /// author and "has file" are applied to the loaded messages.
    public func search(store: BotStore) async {
        sequence += 1
        let request = sequence
        let text = query.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleared = lastQuery.isEmpty && !text.isEmpty
        let changed = lastQuery != text
        lastQuery = text
        guard !text.isEmpty else {
            hits = []; loading = false; spinnerVisible = false; error = nil
            return
        }
        if !hitsAfter(queryChanged: changed, cleared: cleared) { hits = [] }
        loading = true
        spinnerVisible = false
        error = nil
        defer { if request == sequence { loading = false; spinnerVisible = false; spinnerTask?.cancel() } }
        do {
            try await Task.sleep(for: .milliseconds(300))
            // The debounce isn't Searching: the spinner clock starts with the host call.
            let startedAt = Date()
            spinnerTask = Task { [weak self] in
                try? await Task.sleep(for: .milliseconds(200))
                guard let self, request == self.sequence, !Task.isCancelled else { return }
                self.spinnerVisible = showSpinner(startedAt: startedAt, now: Date())
            }
            let client = try await store.ready()
            struct Request: Encodable { var query: String; var botIds: [String]; var limit: Int }
            struct Response: Decodable { var results: [Result] }
            struct Result: Decodable {
                var id: String
                var botId: String
                var threadId: String?
                var text: String
                var createdAt: Int64
            }
            let response: Response = try await client.call("searchMessages",
                                                           Request(query: text, botIds: [botId], limit: 50))
            try Task.checkCancellation()
            guard request == sequence else { return }
            let since = range.since()
            var found: [Hit] = []
            for result in response.results where result.createdAt >= since {
                let hit = Hit(id: result.id, botId: result.botId, threadId: result.threadId,
                              text: result.text, createdAt: result.createdAt)
                // Author and file filters need the message itself; the host returns it in the
                // context of the same search hit, cached so re-filtering stays cheap.
                if needsDetails {
                    guard let entry = try await loadDetail(for: hit, store: store),
                          matchesFilters(entry, store: store) else { continue }
                    guard request == sequence else { return }
                }
                found.append(hit)
            }
            guard request == sequence else { return }
            hits = found
        } catch is CancellationError {
        } catch {
            if request == sequence { self.error = error.localizedDescription }
        }
    }

    private var needsDetails: Bool {
        !author.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || hasFile
    }

    /// The matched entry, read through `messageContext` (the same call roster search uses to jump).
    private func loadDetail(for hit: Hit, store: BotStore) async throws -> Entry? {
        if let cached = details[hit.id] { return cached }
        let entries = try await store.messageContext(botId: hit.botId, entryId: hit.id)
        let entry = entries.first { $0.id == hit.id }
        details[hit.id] = entry
        return entry
    }

    private func matchesFilters(_ entry: Entry, store: BotStore) -> Bool {
        if hasFile && entry.data.files?.isEmpty != false && entry.data.attachments?.isEmpty != false { return false }
        let wanted = author.trimmingCharacters(in: .whitespacesAndNewlines)
        if !wanted.isEmpty {
            let name: String
            if entry.kind == "user" {
                name = entry.data.person?.name ?? "You"
            } else {
                name = entry.data.author.flatMap { store.bots[$0]?.name } ?? entry.data.author ?? ""
            }
            guard name.localizedCaseInsensitiveContains(wanted) else { return false }
        }
        return true
    }

    /// Loads the span around a hit so the view can show the message in place and jump into it.
    public func open(_ hit: Hit, store: BotStore) async {
        let request = sequence
        do {
            let entries = try await store.messageContext(botId: hit.botId, entryId: hit.id)
            try Task.checkCancellation()
            guard request == sequence else { return }
            selected = hit
            context = entries
        } catch is CancellationError {
        } catch { if request == sequence { self.error = error.localizedDescription } }
    }

    public func closeContext() { context = nil; selected = nil }
}
