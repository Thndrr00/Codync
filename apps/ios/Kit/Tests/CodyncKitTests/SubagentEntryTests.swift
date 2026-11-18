import Foundation
import Testing
@testable import CodyncKit

/// A turn's subagents arrive as one chat entry (`kind: "subagents"`), which the card renders: a
/// client that doesn't know the card shows the entry's text instead.
@Test func aSubagentsCardIsChatContentAndSaysWhichSubagentsWork() throws {
    let json = #"""
    {"id":"card","seq":1,"botId":"b","rev":2,"kind":"subagents","turn":1,
     "data":{"text":"Subagents: Review the upload API, Fix the retry","author":"b",
             "subagents":[
               {"id":"t1","task":"Review the upload API","state":"done","step":"Read src/api/bots.rs","startedAt":1000,"endedAt":3000},
               {"id":"t2","task":"Fix the retry","state":"running","step":"","startedAt":2000,"endedAt":null}
             ]},
     "createdAt":1000,"updatedAt":2000}
    """#
    let entry = try JSONDecoder().decode(Entry.self, from: Data(json.utf8))
    #expect(entry.isChat)
    let subagents = entry.data.subagents ?? []
    #expect(subagents.map(\.id) == ["t1", "t2"])
    #expect(subagents.map(\.isRunning) == [false, true])
    #expect(subagents.map(\.shownState) == ["done", "running"])
}

/// A card a host names only partly still shows what it has, and a subagent with no state yet is working.
@Test func aPartialSubagentCardStillDecodes() throws {
    let json = #"{"subagents":[{"task":"Fix the retry","state":"failed"},{"id":"t2"}]}"#
    let data = try JSONDecoder().decode(EntryData.self, from: Data(json.utf8))
    #expect(data.subagents?.map(\.task) == ["Fix the retry", ""])
    #expect(data.subagents?.map(\.shownState) == ["failed", "running"])
    #expect(data.subagents?.allSatisfy({ $0.step.isEmpty && $0.startedAt == 0 && $0.endedAt == nil }) == true)
}
