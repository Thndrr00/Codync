import CodyncKit
import Foundation

extension ChatLibraryItem {
    enum Period: String, CaseIterable {
        case today = "Today", yesterday = "Yesterday", thisWeek = "This week", earlier = "Earlier"
    }

    /// Where a date falls: today, yesterday, the last seven days, or earlier (not the calendar week).
    static func period(for date: Date, now: Date = .now, calendar: Calendar = .current) -> Period {
        if calendar.isDate(date, inSameDayAs: now) { return .today }
        if let yesterday = calendar.date(byAdding: .day, value: -1, to: now), calendar.isDate(date, inSameDayAs: yesterday) {
            return .yesterday
        }
        let weekAgo = calendar.date(byAdding: .day, value: -6, to: calendar.startOfDay(for: now)) ?? now
        return date >= weekAgo ? .thisWeek : .earlier
    }

    /// The Library's groups by when each item was published or shared, newest first; empty groups left out.
    static func sections(_ items: [ChatLibraryItem], now: Date = .now, calendar: Calendar = .current)
        -> [(period: Period, items: [ChatLibraryItem])] {
        let grouped = Dictionary(grouping: items) { period(for: $0.date, now: now, calendar: calendar) }
        return Period.allCases.compactMap { period in
            grouped[period].map { (period, $0.sorted { $0.at > $1.at }) }
        }
    }
}

extension ServerLibraryItem {
    /// The server's Library in the same groups as a chat's, newest first; empty groups left out.
    static func sections(_ items: [ServerLibraryItem], now: Date = .now, calendar: Calendar = .current)
        -> [(period: ChatLibraryItem.Period, items: [ServerLibraryItem])] {
        let grouped = Dictionary(grouping: items) { ChatLibraryItem.period(for: $0.date, now: now, calendar: calendar) }
        return ChatLibraryItem.Period.allCases.compactMap { period in
            grouped[period].map { (period, $0.sorted { $0.at > $1.at }) }
        }
    }
}
