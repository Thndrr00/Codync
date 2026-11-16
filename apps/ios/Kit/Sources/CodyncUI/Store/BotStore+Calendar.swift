import CodyncKit
import Foundation

/// A read-only calendar feed (`.ics`) this computer subscribes to.
public struct CalendarFeed: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var name: String
    public var url: String
    public var fetchedAt: Int?

    public init(id: String, name: String, url: String, fetchedAt: Int? = nil) {
        self.id = id
        self.name = name
        self.url = url
        self.fetchedAt = fetchedAt
    }
}

/// One event from a subscribed calendar; times are Unix milliseconds.
public struct CalendarEvent: Decodable, Identifiable, Equatable, Sendable {
    public var feed: String
    public var title: String
    public var start: Int
    public var end: Int?
    public var allDay: Bool
    public var location: String?
    public var url: String?

    public var id: String { "\(feed)-\(start)-\(title)" }
}

extension BotStore {
    private struct FeedsWrap: Decodable { var feeds: [CalendarFeed] }
    private struct EventsWrap: Decodable { var events: [CalendarEvent] }
    private struct FeedWrap: Decodable { var feed: CalendarFeed }

    public func calendarFeeds() async throws -> [CalendarFeed] {
        let client = try await ready()
        let wrap: FeedsWrap = try await client.call("calendarFeeds")
        return wrap.feeds
    }

    /// Adds a feed and returns it once the computer has read it.
    public func addCalendarFeed(name: String, url: String) async throws -> CalendarFeed {
        let client = try await ready()
        let wrap: FeedWrap = try await client.call("addCalendarFeed", ["name": name, "url": url], timeout: 45)
        return wrap.feed
    }

    @discardableResult
    public func removeCalendarFeed(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var removed: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("removeCalendarFeed", ["id": id])
        return wrap.removed
    }

    /// The next `days` of events across every feed.
    public func calendarAgenda(days: Int = 14) async throws -> [CalendarEvent] {
        let client = try await ready()
        let wrap: EventsWrap = try await client.call("calendarAgenda", ["days": days])
        return wrap.events
    }
}
