import Foundation
import Testing
@testable import CodyncKit

@Test func the_app_bridge_decodes_the_host_envelope_and_keeps_its_sandbox_policy() throws {
    let json = #"{"status":200,"contentType":"text/html; charset=utf-8","csp":"sandbox allow-scripts","body":"PGgxPkhpPC9oMT4="}"#
    let wire = try JSONDecoder().decode(AppRequestWire.self, from: Data(json.utf8))
    let response = try HostClient.decodeAppResponse(wire)

    #expect(response.status == 200)
    #expect(response.contentType.contains("text/html"))
    #expect(response.csp == "sandbox allow-scripts")
    #expect(String(data: response.body, encoding: .utf8) == "<h1>Hi</h1>")
}

@Test func a_broken_body_is_rejected_instead_of_rendering_empty_html() {
    let json = #"{"status":200,"contentType":"text/html","csp":"sandbox","body":"not base64 %"}"#
    let wire = try? JSONDecoder().decode(AppRequestWire.self, from: Data(json.utf8))
    #expect(wire != nil)
    #expect(throws: AppBridgeError.invalidBody) {
        try HostClient.decodeAppResponse(wire!)
    }
}

@Test func the_list_apps_answer_feeds_the_wire_models_directly() throws {
    let json = #"{"apps":[{"slug":"kanban","botId":"b1","title":"Board","version":2,"url":"/apps/b1/kanban/tok/","links":[{"token":"tok","botId":"b1","slug":"kanban","mode":"edit"}]}]}"#
    struct Res: Decodable { var apps: [BotApp] }
    let apps = try JSONDecoder().decode(Res.self, from: Data(json.utf8)).apps
    #expect(apps.count == 1)
    #expect(apps.first?.links.first?.mode == .edit)
    #expect(apps.first?.id == "kanban")
}

@Test func a_created_link_carries_the_full_url_and_permission() throws {
    let json = #"{"link":{"token":"tok","botId":"b1","slug":"kanban","mode":"view"},"url":"https://host/apps/b1/kanban/tok/"}"#
    let created = try JSONDecoder().decode(CreatedAppLink.self, from: Data(json.utf8))
    #expect(created.link.mode == .view)
    #expect(created.url.hasSuffix("/tok/"))
}
