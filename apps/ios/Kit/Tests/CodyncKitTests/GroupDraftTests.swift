import Foundation
import Testing
@testable import CodyncKit

/// The group editor's draft carries the channel fields the roster and the picker read, and clearing
/// the title or the lead sends `null` so the host clears it too.
@Test func aGroupDraftEncodesItsTitleLeadAndArchive() throws {
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"""
    {"id":"g","kind":"group","name":"Crew","members":["a"],"title":"Ships 2.0","archived":true,"chiefBotId":"a"}
    """#.utf8))
    #expect(bot.title == "Ships 2.0")
    #expect(bot.archived == true)
    #expect(bot.chiefBotId == "a")

    var draft = GroupDraft(bot)
    draft.archived = false
    var object = try JSONSerialization.jsonObject(with: JSONEncoder().encode(draft))
    var json = try #require(object as? [String: Any])
    #expect(json["title"] as? String == "Ships 2.0")
    #expect(json["chiefBotId"] as? String == "a")
    #expect(json["archived"] as? Bool == false)

    draft.title = nil
    draft.chiefBotId = nil
    object = try JSONSerialization.jsonObject(with: JSONEncoder().encode(draft))
    json = try #require(object as? [String: Any])
    #expect(json["title"] is NSNull)
    #expect(json["chiefBotId"] is NSNull)
}
