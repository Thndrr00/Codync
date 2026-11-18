import Foundation
import Testing
@testable import CodyncKit
@testable import CodyncUI

/// An archived group leaves the open roster, stays in `archivedGroups`, and comes back when restored.
@MainActor @Test func archivedGroupsLeaveTheOpenRoster() throws {
    let suite = "ArchivedGroupTests.\(UUID().uuidString)"
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let storage = SharedStore.Context(accountID: UUID().uuidString, suite: suite)
    let store = BotStore(computer: Computer(id: UUID().uuidString, name: "Mac", signKey: "test"), storage: storage) { _ in
        FakeRemote(.connecting)
    }
    defer { store.retire() }
    let decode = { (json: String) in try JSONDecoder().decode(Bot.self, from: Data(json.utf8)) }
    store.bots["g"] = try decode(#"{"id":"g","kind":"group","name":"Crew","members":["a"],"archived":true}"#)
    store.bots["open"] = try decode(#"{"id":"open","kind":"group","name":"Open","members":["a"]}"#)
    #expect(store.roster.map(\.id) == ["open"])
    #expect(store.archivedGroups.map(\.id) == ["g"])
    store.bots["g"]?.archived = false
    #expect(store.roster.map(\.id).sorted() == ["g", "open"])
    #expect(store.archivedGroups.isEmpty)
}
