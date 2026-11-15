import Foundation
import Testing
@testable import CodyncKit

private let missionJSON = #"{"id":"m1","botId":"b1","title":"Ship it","goal":"by Friday","status":"waiting","checkpoint":"step one done","createdAt":1,"updatedAt":2,"steps":[{"id":"s1","missionId":"m1","position":0,"text":"write it","status":"done","at":1},{"id":"s2","missionId":"m1","position":1,"text":"review it","status":"pending","at":2}]}"#

@Test func readsAMissionWithItsSteps() throws {
    let mission = try JSONDecoder().decode(Mission.self, from: Data(missionJSON.utf8))
    #expect(mission.status == .waiting && mission.checkpoint == "step one done")
    #expect(mission.steps.map(\.text) == ["write it", "review it"])
    #expect(mission.steps[0].status == .done && mission.steps[1].status.mark == "·")
    #expect(mission.stopReason == nil)
}

@Test func onlyAFinalMissionCountsAsReturned() {
    #expect(MissionStatus.allCases.filter(\.isReturned) == [.done, .abandoned])
}

/// Every move the sheet offers: on from proposed or a wait, back from a finished one, paused from running.
@Test func theNextMoveOfEachStatus() {
    #expect(MissionStatus.proposed.next == .active)
    #expect(MissionStatus.waiting.next == .active)
    #expect(MissionStatus.done.next == .active)
    #expect(MissionStatus.abandoned.next == .active)
    #expect(MissionStatus.active.next == .waiting)
    #expect(MissionStatus.proposed.nextTitle == L10n.tr("Activate"))
    #expect(MissionStatus.active.nextTitle == L10n.tr("Pause"))
}

@Test func anUnknownStatusIsAStatusTheAppCannotRead() {
    #expect(throws: DecodingError.self) {
        _ = try JSONDecoder().decode(Mission.self, from: Data(#"{"id":"m","status":"nearly"}"#.utf8))
    }
}
