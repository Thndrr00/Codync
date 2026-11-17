import CodyncKit
import Foundation

/// Something other people owe the person: a reply, a review, a delivery. Kept on this computer and
/// repeated in the daily digest.
public struct WaitingItem: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var title: String
    public var counterpart: String
    public var url: String
    public var note: String
    public var dueAt: Int?
    public var done: Bool
    public var createdAt: Int
    public var updatedAt: Int
}

extension BotStore {
    private struct SaveBody: Encodable {
        var title: String
        var counterpart: String
        var dueAt: Int?
    }
    private struct DoneBody: Encodable {
        var id: String
        var done: Bool
    }
    private struct ItemsWrap: Decodable { var items: [WaitingItem] }
    private struct ItemWrap: Decodable { var item: WaitingItem }

    public func waitingItems() async throws -> [WaitingItem] {
        let client = try await ready()
        let wrap: ItemsWrap = try await client.call("waitingItems")
        return wrap.items
    }

    @discardableResult
    public func saveWaitingItem(title: String, counterpart: String, dueAt: Int?) async throws -> WaitingItem {
        let client = try await ready()
        let wrap: ItemWrap = try await client.call(
            "saveWaitingItem",
            SaveBody(title: title, counterpart: counterpart, dueAt: dueAt)
        )
        return wrap.item
    }

    @discardableResult
    public func setWaitingDone(_ id: String, done: Bool) async throws -> WaitingItem {
        let client = try await ready()
        let wrap: ItemWrap = try await client.call("setWaitingDone", DoneBody(id: id, done: done))
        return wrap.item
    }

    @discardableResult
    public func deleteWaitingItem(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var deleted: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("deleteWaitingItem", ["id": id])
        return wrap.deleted
    }
}
