import Foundation
import Testing
@testable import CodyncKit

/// A channel's request tree arrives on its bot, each row with the member it belongs to and how it is going.
@Test func aGroupCarriesItsTaskTree() throws {
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"""
    {"id":"g","kind":"group","name":"Crew","members":["a"],
     "tasks":[{"id":"t1","groupId":"g","lane":"g","requestId":"u1","rootId":"t1","ownerId":"a","ownerName":"Ada",
               "kind":"assignment","state":"running","instruction":"Look into it","createdAt":10,"updatedAt":11}]}
    """#.utf8))
    let task = try #require(bot.tasks.first)
    #expect(bot.tasks.count == 1)
    #expect(task.ownerName == "Ada")
    #expect(task.state == "running")
    #expect(task.isOpen)
    #expect(!task.isStopped)
}

/// A bot from a host before the tree, or one that never had a request, has none.
@Test func aBotWithoutTasksHasAnEmptyTree() throws {
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b","kind":"agent","name":"Solo"}"#.utf8))
    #expect(bot.tasks.isEmpty)
}

/// A row an older host sends with only what it has still shows: the id is all it must carry.
@Test func aTaskRowToleratesMissingFields() throws {
    let task = try JSONDecoder().decode(GroupTask.self, from: Data(#"{"id":"t1","state":"paused"}"#.utf8))
    #expect(task.groupId.isEmpty)
    #expect(task.instruction.isEmpty)
    #expect(task.isStopped)
    #expect(task.createdAt == task.updatedAt)
}
