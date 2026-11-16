import Foundation
import Testing
@testable import CodyncKit

@Test func proactiveSettingsFillDefaultsFromAnOlderHost() throws {
    let settings = try JSONDecoder().decode(ProactiveSettings.self, from: Data(#"{"enabled":true,"everyHours":6}"#.utf8))
    #expect(settings.enabled)
    #expect(settings.everyHours == 6)
    #expect(settings.quietStart == 22)
    #expect(settings.quietEnd == 8)
    #expect(settings.maxPerDay == 3)
    #expect(settings.appsInBackground)

    let full = try JSONDecoder().decode(
        ProactiveSettings.self,
        from: Data(#"{"quietStart":23,"quietEnd":7,"timezone":"America/New_York","maxPerDay":2,"appsInBackground":false}"#.utf8)
    )
    #expect(full.quietStart == 23)
    #expect(full.timezone == "America/New_York")
    #expect(!full.appsInBackground)
}

@Test func appLinksAndMiniAppsDecodeWithCollections() throws {
    let data = Data(
        #"{"slug":"kanban","botId":"b1","title":"Board","entry":"index.html","version":3,"url":"/apps/b1/kanban/tok/","links":[{"token":"tok","botId":"b1","slug":"kanban","label":"Team","mode":"edit","opens":2}],"collections":[{"collection":"tasks","count":4,"updatedAt":42}]}"#.utf8
    )
    let app = try JSONDecoder().decode(BotApp.self, from: data)
    #expect(app.id == "kanban")
    #expect(app.version == 3)
    #expect(app.links.first?.mode == .edit)
    #expect(app.links.first?.id == "tok")
    #expect(app.collections.first?.count == 4)
    #expect(app.description == nil)
}

@Test func openItemsAndConnectorFailuresMatchTheHostShapes() throws {
    let item = try JSONDecoder().decode(
        OpenItem.self,
        from: Data(#"{"id":"item_1","botId":"b1","title":"Waiting on invoice","checkAfter":1790000000000}"#.utf8)
    )
    #expect(item.status == "open")
    #expect(item.note.isEmpty)
    #expect(item.checkAfter == 1_790_000_000_000)

    let failure = try JSONDecoder().decode(
        ConnectorFailure.self,
        from: Data(#"{"kind":"auth","message":"token expired","at":1790000000000}"#.utf8)
    )
    #expect(failure.kind == "auth")
    #expect(failure.at == 1_790_000_000_000)
}

@Test func actionClassesAndAvatarLooksListEveryChoice() {
    #expect(ActionClass.allCases.count == 7)
    #expect(ActionClass.catalog.map(\.id) == ActionClass.allCases)
    #expect(ActionClass.catalog.last?.id == .routines)
    #expect(RobotLook.allCases.count == 12)
    #expect(RobotLook.catalog.first?.label == "Round")
    #expect(RobotLook.catalog.last?.id == .hex)
    #expect((try? JSONDecoder().decode(ActionClass.self, from: Data(#""apps""#.utf8))) == .apps)
}
