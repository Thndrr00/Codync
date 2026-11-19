import Foundation
import Testing
@testable import CodyncKit

@Test func localAppsReadTheComputersCountLimitAndChoices() throws {
    let json = #"{"count":7,"limit":250,"choices":[25,50,100,250,1000],"botsWithApps":3}"#
    let apps = try JSONDecoder().decode(LocalApps.self, from: Data(json.utf8))
    #expect(apps.count == 7)
    #expect(apps.limit == 250)
    #expect(apps.botsWithApps == 3)
    #expect(apps.choices == [25, 50, 100, 250, 1_000])
}

@Test func localAppsKeepTheCardWhenAFieldIsMissing() throws {
    let partial = try JSONDecoder().decode(LocalApps.self, from: Data(#"{"count":2,"limit":50}"#.utf8))
    #expect(partial.count == 2)
    #expect(partial.limit == 50)
    #expect(partial.botsWithApps == 0)
    #expect(partial.choices == LocalApps.defaultChoices, "an older host offers nothing: the same values stand")

    let empty = try JSONDecoder().decode(LocalApps.self, from: Data("{}".utf8))
    #expect(empty.count == 0)
    #expect(empty.limit == LocalApps.defaultLimit)
}
