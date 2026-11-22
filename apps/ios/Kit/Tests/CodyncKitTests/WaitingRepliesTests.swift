import Foundation
import Testing
@testable import CodyncKit

private func asked(_ id: String, status: String, thread: String? = nil) throws -> Entry {
    let json = """
    {"id":"\(id)","botId":"egan","seq":1,"rev":1,"kind":"notice","createdAt":1,"updatedAt":1,
    "data":{"text":"Asked Owen: \(id)","heading":"Asked Owen: \(id)","status":"\(status)",
    "botMessage":{"sourceBotId":"egan","targetBotId":"owen","text":"Inspect \(id)"}}}
    """
    var entry = try JSONDecoder().decode(Entry.self, from: Data(json.utf8))
    entry.threadId = thread
    return entry
}

@Test func onlyPendingAskedRowsWaitAndAStoppedRowKeepsItsPlace() throws {
    let rows = WaitingReplies.rows(
        [try asked("waiting", status: "queued"), try asked("done", status: "completed"), try asked("stopped", status: "cancelled")],
        botId: "egan", threadId: nil, stopped: ["stopped"], name: { "Name:\($0)" }
    )
    #expect(rows.map(\.id) == ["stopped", "waiting"])
    #expect(rows.map(\.state) == [.stopped, .asked])
    #expect(rows.first?.name == "Name:owen")
    #expect(rows.first?.request == "Inspect stopped")
}

@Test func aMessageOrAnotherThreadsAskDoesNotWait() throws {
    var message = try asked("message", status: "queued")
    message.data.heading = "Messaged Owen: hi"
    let rows = WaitingReplies.rows(
        [message, try asked("done", status: "completed"), try asked("thread", status: "queued", thread: "other"),
         try asked("mine", status: "queued")],
        botId: "egan", threadId: nil, stopped: [], name: { $0 }
    )
    #expect(rows.map(\.id) == ["mine"])
}
