import Testing
@testable import CodyncUI
@testable import CodyncKit

/// The roster's "Mark unread" decision, pure so it can be checked without a host.
@Test func thereIsNoUnreadActionWithoutAMessage() {
    #expect(unreadAction(nil) == nil)
    #expect(unreadAction("e3") == UnreadAction(entryId: "e3"))
}

@Test func theNewestFinalAgentReplyIsWhatTheRosterMarksUnread() {
    let entries = [chatEntry("u1", "user"), chatEntry("a1", "agent"), chatEntry("t1", "thought"), chatEntry("a2", "agent")]
    #expect(unreadEntryId(entries) == "a2")
    #expect(unreadEntryId([chatEntry("u1", "user")]) == nil)
    #expect(unreadEntryId([]) == nil)
}

private func chatEntry(_ id: String, _ kind: String) -> Entry {
    var data = EntryData()
    data.final = true
    return Entry(id: id, seq: 0, botId: "bot", rev: 0, kind: kind, turn: 0, data: data, createdAt: 0, updatedAt: 0)
}
