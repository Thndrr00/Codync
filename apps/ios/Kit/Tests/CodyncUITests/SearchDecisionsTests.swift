import Foundation
import Testing
@testable import CodyncUI

/// OpenBot 004's search motion and responsiveness decisions, pure so they can be checked without a host.
@Test func reduceMotionAndNoneSwitchSearchRowsOff() {
    #expect(searchMotion(.fade, reduce: false) == .fade)
    #expect(searchMotion(.blur, reduce: false) == .blur)
    #expect(searchMotion(.none, reduce: false) == .none)
    #expect(searchMotion(.fade, reduce: true) == .none)
    #expect(searchMotion(.blur, reduce: true) == .none)
}

@Test func theSearchingSpinnerWaitsForTheHost() {
    let started = Date(timeIntervalSince1970: 1_000)
    #expect(!showSpinner(startedAt: nil, now: started))
    #expect(!showSpinner(startedAt: started, now: started.addingTimeInterval(0.19)))
    #expect(showSpinner(startedAt: started, now: started.addingTimeInterval(0.2)))
    #expect(!showSpinner(startedAt: started, now: started.addingTimeInterval(0.29), delayMs: 0.3))
    #expect(showSpinner(startedAt: started, now: started.addingTimeInterval(0.3), delayMs: 0.3))
}

@Test func priorHitsStayWhileAnEditedQueryLoads() {
    #expect(hitsAfter(queryChanged: false, cleared: false))
    #expect(hitsAfter(queryChanged: true, cleared: false))
    #expect(!hitsAfter(queryChanged: true, cleared: true))
    #expect(hitsAfter(queryChanged: false, cleared: true))
}

@Test func theSearchMotionLabelsNameTheThreeSettingsOptions() {
    #expect(SearchMotionStyle.title == "Animate search results")
    #expect(SearchMotionStyle.allCases.map(\.label) == ["Blur", "Fade", "None"])
    #expect(SearchMotionStyle.allCases.map(\.rawValue) == ["blur", "fade", "none"])
}

@Test func theSettingsRowOffersThreeOptionsInOrder() {
    let options = searchMotionOptions()
    #expect(options.map(\.id) == [.blur, .fade, .none])
    #expect(options.map(\.label) == ["Blur", "Fade", "None"])
    #expect(SearchMotionStyle.storageKey == "codync.animateSearchResults")
}
