import Foundation
import Testing
@testable import CodyncKit

@Test func antigravityAccountsDecodeWithoutFabricatedUsage() throws {
    let data = Data(#"{"providers":[{"id":"antigravity:acc_one","name":"Antigravity · One","source":"antigravity","updatedAt":1,"status":"unsupported","windows":[]},{"id":"antigravity:acc_two","name":"Antigravity · Two","source":"antigravity","updatedAt":2,"status":"unavailable","windows":[]}]}"#.utf8)
    let report = try JSONDecoder().decode(Usage.self, from: data)
    #expect(report.providers.count == 2)
    #expect(report.providers[0].tightest == nil)
    #expect(report.providers[0].statusLabel == "Update this agent to read its plan limits.")
    #expect(report.providers[1].statusLabel == "Plan limits are unavailable for this account.")
    #expect(report.providers[0].registry == "antigravity-acp")
}

@Test func quotaTitlesKeepTheirModelGroup() throws {
    let data = Data(#"{"id":"g","label":"Gemini Models · Weekly Limit","percent":4}"#.utf8)
    let window = try JSONDecoder().decode(UsageWindow.self, from: data)
    #expect(window.title == "Gemini Models · Weekly Limit")
}

/// The model picker's chip and each stopped model's label read this: a spent cap, not a balance, and
/// only until its reset comes.
@Test func aSpentWindowStopsEveryModelUntilItsReset() throws {
    let now = Date(timeIntervalSince1970: 1_700_000_000)
    let json = #"""
    {"providers":[
      {"id":"claude:acc_one","name":"Claude · One","source":"claude","updatedAt":1,"windows":[
        {"id":"five_hour","label":"5-hour","percent":100,"resetsAt":1700003600000},
        {"id":"weekly","label":"Weekly","percent":40,"resetsAt":1700604800000}]},
      {"id":"codex","name":"Codex","source":"codex","updatedAt":2,"windows":[
        {"id":"weekly","label":"Weekly","percent":100,"resetsAt":1600000000000},
        {"id":"balance","label":"Balance","percent":100,"text":"$4.20"}]}
    ]}
    """#
    let report = try JSONDecoder().decode(Usage.self, from: Data(json.utf8))
    // An account's provider id carries its account after a colon.
    #expect(report.spentWindow(for: "claude", now: now)?.id == "five_hour")
    // A window whose reset already came stops nothing, and neither does a reading that isn't a cap.
    #expect(report.spentWindow(for: "codex", now: now) == nil)
    #expect(report.spentWindow(for: "gemini", now: now) == nil)
    #expect(report.spentWindow(for: "claude", now: Date(timeIntervalSince1970: 1_700_010_000)) == nil)
}
