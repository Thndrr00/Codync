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
