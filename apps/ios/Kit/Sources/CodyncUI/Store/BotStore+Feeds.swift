import CodyncKit
import Foundation

/// An RSS or Atom feed this computer follows.
public struct NewsFeed: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var name: String
    public var url: String
    public var items: Int
    public var fetchedAt: Int?
}

/// One headline from a feed.
public struct NewsItem: Decodable, Identifiable, Equatable, Sendable {
    public var feed: String
    public var title: String
    public var url: String?
    public var published: Int?
    public var summary: String

    public var id: String { "\(feed)-\(url ?? title)-\(published ?? 0)" }
}

extension BotStore {
    private struct FeedsWrap: Decodable { var feeds: [NewsFeed] }
    private struct ItemsWrap: Decodable { var items: [NewsItem] }
    private struct AddBody: Encodable {
        var name: String
        var url: String
    }
    private struct ItemsBody: Encodable {
        var limit: Int
    }

    public func newsFeeds() async throws -> [NewsFeed] {
        let client = try await ready()
        let wrap: FeedsWrap = try await client.call("feedSubscriptions")
        return wrap.feeds
    }

    @discardableResult
    public func addNewsFeed(name: String, url: String) async throws -> Int {
        struct Wrap: Decodable { var items: Int }
        let client = try await ready()
        let wrap: Wrap = try await client.call("addFeed", AddBody(name: name, url: url), timeout: 45)
        return wrap.items
    }

    @discardableResult
    public func removeNewsFeed(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var removed: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("removeFeed", ["id": id])
        return wrap.removed
    }

    public func newsItems(limit: Int = 60) async throws -> [NewsItem] {
        let client = try await ready()
        let wrap: ItemsWrap = try await client.call("feedItems", ItemsBody(limit: limit), timeout: 45)
        return wrap.items
    }
}
