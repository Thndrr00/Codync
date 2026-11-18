import Foundation
import Testing
@testable import CodyncKit
@testable import CodyncUI

private func message(_ id: String, text: String = "Ship the release notes") -> Entry {
    Entry(id: id, seq: 1, botId: "b1", rev: 1, kind: "user", turn: 1, data: EntryData(text: text),
          createdAt: 1, updatedAt: 1)
}

@MainActor @Test func forwardingSendsTheMessageOnToTheChosenChat() async throws {
    let (storage, suite) = context()
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let fake = FakeRemote(.ready(.direct))
    let store = BotStore(computer: randomComputer("Mac"), storage: storage) { _ in fake }
    defer { store.retire() }
    store.setActive(true)
    #expect(await until { await fake.subscribed })
    await fake.emit(botEvent("b1", name: "Alice", rev: 1))
    await fake.emit(botEvent("g1", name: "Ship room", rev: 2))
    #expect(await until { store.connection == .online })

    let source = message("u1")
    try await store.forward(source, to: "g1")

    // The computer gets the message's id and the chat picked, and the target shows the copy it made.
    #expect(await fake.forwards == [ForwardCall(entryId: "u1", botId: "g1")])
    #expect(store.chat("g1").contains { $0.id == "f1" && $0.botId == "g1" })
    #expect(store.chat("b1").allSatisfy { $0.id != "f1" }, "the source chat keeps only its own message")
}

@Test func aForwardedCopyKnowsWhereItCameFrom() throws {
    let json = #"{"id":"f1","botId":"g1","seq":1,"rev":1,"kind":"user","turn":1,"data":{"text":"Notes","forwardedFrom":{"botId":"b1","entryId":"u1","author":"Alice"}},"createdAt":1,"updatedAt":1}"#
    let entry = try JSONDecoder().decode(Entry.self, from: Data(json.utf8))
    #expect(entry.data.forwardedFrom?.botId == "b1")
    #expect(entry.data.forwardedFrom?.entryId == "u1")
    #expect(entry.data.forwardedFrom?.author == "Alice")
}

@Test func aPartialForwardedCopyStillDecodes() throws {
    let json = #"{"id":"f1","botId":"g1","seq":1,"rev":1,"kind":"user","turn":1,"data":{"text":"Notes","forwardedFrom":{}},"createdAt":1,"updatedAt":1}"#
    let entry = try JSONDecoder().decode(Entry.self, from: Data(json.utf8))
    #expect(entry.data.forwardedFrom?.author == nil)
    #expect(entry.data.text == "Notes")
}
