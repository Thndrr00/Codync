import Foundation
import Testing
@testable import CodyncKit

/// The group editor's draft carries the channel fields the roster and the picker read, and clearing
/// the title, the lead or the picture sends `null` so the host clears it too.
@Test func aGroupDraftEncodesItsTitleLeadArchiveAndPicture() throws {
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"""
    {"id":"g","kind":"group","name":"Crew","members":["a"],"title":"Ships 2.0","archived":true,"chiefBotId":"a",
     "avatarPhoto":"data:image/png;base64,AA=="}
    """#.utf8))
    #expect(bot.title == "Ships 2.0")
    #expect(bot.archived == true)
    #expect(bot.chiefBotId == "a")
    #expect(bot.avatarPhoto == "data:image/png;base64,AA==")

    var draft = GroupDraft(bot)
    draft.archived = false
    var object = try JSONSerialization.jsonObject(with: JSONEncoder().encode(draft))
    var json = try #require(object as? [String: Any])
    #expect(json["title"] as? String == "Ships 2.0")
    #expect(json["chiefBotId"] as? String == "a")
    #expect(json["archived"] as? Bool == false)
    #expect(json["avatarPhoto"] as? String == "data:image/png;base64,AA==")

    draft.title = nil
    draft.chiefBotId = nil
    draft.avatarPhoto = nil
    object = try JSONSerialization.jsonObject(with: JSONEncoder().encode(draft))
    json = try #require(object as? [String: Any])
    #expect(json["title"] is NSNull)
    #expect(json["chiefBotId"] is NSNull)
    #expect(json["avatarPhoto"] is NSNull)
}
