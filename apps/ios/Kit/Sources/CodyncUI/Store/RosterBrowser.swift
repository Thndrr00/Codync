import CodyncKit
import Foundation
import Observation

@MainActor @Observable public final class RosterBrowser {
    public struct Match: Identifiable {
        public var id: String { "\(store.computer.id):\(hit.id)" }
        public let store: BotStore
        public let hit: RosterSearchHit
    }

    public private(set) var matches: [Match] = []
    public private(set) var loading = false
    public private(set) var error: String?
    public private(set) var canLoadMore = false
    public private(set) var context: [Entry]?
    public private(set) var selected: Match?
    private var sequence = 0
    private var cursors: [String: Int64] = [:]
    private var exhausted: Set<String> = []

    public init() {}

    public func search(_ query: String, stores: [BotStore], more: Bool = false) async {
        sequence += 1
        let request = sequence
        if !more { matches = []; cursors = [:]; exhausted = []; canLoadMore = false }
        error = nil
        guard !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { loading = false; return }
        loading = true
        defer { if sequence == request { loading = false } }
        do {
            if !more { try await Task.sleep(for: .milliseconds(300)) }
            var found: [Match] = []
            for store in stores where !exhausted.contains(store.computer.id) {
                let ids = store.roster.map(\.id)
                if ids.isEmpty { exhausted.insert(store.computer.id); continue }
                let page = try await store.searchRoster(query: query, bots: ids, before: cursors[store.computer.id])
                try Task.checkCancellation()
                guard sequence == request else { return }
                found += page.results.map { Match(store: store, hit: $0) }
                if let next = page.next { cursors[store.computer.id] = next }
                else { exhausted.insert(store.computer.id) }
            }
            guard sequence == request else { return }
            matches += found
            canLoadMore = stores.contains { !exhausted.contains($0.computer.id) }
        } catch is CancellationError { }
        catch { if sequence == request { self.error = "Search failed: \(error.localizedDescription)" } }
    }

    public func open(_ match: Match) async {
        loading = true
        defer { loading = false }
        do {
            let entries = try await match.store.rosterContext(match.hit)
            try Task.checkCancellation()
            selected = match
            context = entries
        } catch is CancellationError { }
        catch { self.error = error.localizedDescription }
    }

    public func closeContext() { context = nil; selected = nil }
}
