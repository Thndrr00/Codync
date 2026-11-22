import CodyncKit
import Foundation

/// The archived-thread drawer shows 50 rows at a time; another tap loads the older page.
enum ThreadPaging {
    static let pageSize = 50

    /// Merges a host page into the rows already shown, without dropping a saved thread's messages.
    /// An older host that ignores `before` keeps returning the same rows, so paging stops when
    /// nothing new arrives instead of offering a button that never advances.
    static func merge(_ existing: [ThreadInfo], _ incoming: [ThreadInfo]) -> (threads: [ThreadInfo], hasMore: Bool) {
        var known = Set(existing.map(\.id))
        let added = incoming.filter { thread in
            guard !known.contains(thread.id) else { return false }
            known.insert(thread.id)
            return true
        }
        let merged = (existing + added).sorted { $0.updatedAt > $1.updatedAt }
        return (merged, incoming.count >= pageSize && !added.isEmpty)
    }

    /// The cursor for the next page: the oldest row already shown.
    static func cursor(_ threads: [ThreadInfo]) -> String? { threads.last?.id }
}
