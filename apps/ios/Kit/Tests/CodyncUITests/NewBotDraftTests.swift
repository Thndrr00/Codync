import Testing
@testable import CodyncUI
import CodyncKit

@Test func aNewBotDraftKeepsTheChosenModelAndEffort() {
    var saved = BotDraft()
    saved.model = "gpt-5"
    saved.modelOptions = ["effort": "high"]
    let kept = NewBotDraft.keep(saved: saved, fresh: BotDraft())
    #expect(kept.model == "gpt-5")
    #expect(kept.modelOptions["effort"] == "high")
}

@Test func aFreshChoiceWinsAndAnEmptySavedDraftChangesNothing() {
    var saved = BotDraft()
    saved.model = "old-model"
    saved.modelOptions = ["effort": "low"]
    var fresh = BotDraft()
    fresh.model = "new-model"
    fresh.modelOptions = ["effort": "high"]
    let kept = NewBotDraft.keep(saved: saved, fresh: fresh)
    #expect(kept.model == "new-model")
    #expect(kept.modelOptions["effort"] == "high")
    #expect(NewBotDraft.keep(saved: nil, fresh: fresh) == fresh)
}
