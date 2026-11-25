import Foundation
import Testing
@testable import CodyncKit
@testable import CodyncUI

/// OpenBot 010: the branch menu follows the last finished turn, and branches nest under their source.
@Test func branchIsOfferedOnlyOnTheLastFinalAnswerOfAnIdleTurn() {
    func message(_ seq: Int64, kind: String = "agent", final: Bool? = true) -> Entry {
        var data = EntryData(text: "message")
        data.final = final
        return Entry(id: "e\(seq)", seq: seq, botId: "b", rev: seq, kind: kind, turn: 1, data: data,
                     createdAt: 0, updatedAt: 0)
    }
    let final = message(3)
    #expect(branchAllowed(entry: final, last: final, status: "idle"))
    #expect(!branchAllowed(entry: final, last: message(4), status: "idle"))
    #expect(!branchAllowed(entry: message(3, kind: "user"), last: message(3, kind: "user"), status: "idle"))
    #expect(!branchAllowed(entry: message(3, final: false), last: message(3, final: false), status: "idle"))
    #expect(!branchAllowed(entry: final, last: final, status: "working"))
}

@Test func branchesSharingASourceNestAndALoneBranchStaysFlat() {
    func thread(_ id: String, from: String? = nil) -> ThreadInfo {
        ThreadInfo(id: id, title: "", fromEntry: from, branchText: from == nil ? nil : "Why?", archived: false,
                   createdAt: 0, updatedAt: 0, unread: 0, lastText: nil, lastAuthor: nil)
    }
    let groups = nestThreads([thread("a", from: "root"), thread("b", from: "root"),
                              thread("c"), thread("d", from: "other")])
    #expect(groups.map { $0.children.map(\.id) } == [["a", "b"], ["c"], ["d"]])
    #expect(groups[0].source == "root")
    #expect(groups[0].sourceText == "Why?")
    #expect(groups[1].source == nil)
}
