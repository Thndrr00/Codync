import Foundation
import Testing
@testable import CodyncKit

private func model(_ id: String, _ kinds: [MediaKind], _ caps: MediaCapabilities = MediaCapabilities(
    aspects: ["1:1", "16:9"], durations: [], resolutions: [], transparent: false, voices: nil)) -> MediaModel {
    MediaModel(id: id, name: id, maker: "maker", subscription: "plan", kinds: kinds, aspects: ["1:1", "16:9"],
               transparent: false, voices: false, capabilities: caps)
}

private let answer = MediaAnswer(agent: "grok", installed: true, models: [
    model("grok-imagine", [.image, .video], MediaCapabilities(
        aspects: ["1:1", "16:9"], durations: [6, 10], resolutions: ["480p", "720p"], transparent: true,
        voices: [MediaVoice(id: "ara", name: "Ara")])),
])

@Test func aKindOffersOnlyTheModelsThatMakeItStartingOnTheFirst() {
    #expect(answer.models(of: .image).map(\.id) == ["grok-imagine"])
    #expect(MediaAnswer(agent: "codex", installed: true, models: [model("gpt-image", [.image])]).models(of: .video).isEmpty)
    let none: MediaAnswer? = nil
    #expect(none.firstChoice(.image) == nil)
    #expect(answer.firstChoice(.video) == MediaChoice(kind: .video, model: "grok-imagine", aspect: "1:1", duration: 6,
                                                     resolution: "480p", transparent: false, voice: nil))
}

@Test func anotherModelKeepsOnlyTheSettingsItMakes() {
    let choice = MediaChoice(kind: .video, model: "grok-imagine", aspect: "16:9", duration: 10, resolution: "720p",
                             transparent: true, voice: "ara")
    let moved = choice.on(model("other", [.video]))
    #expect(moved.model == "other")
    #expect(moved.aspect == "1:1")
    #expect(moved.duration == 0)
    #expect(moved.voice == nil)
    #expect(moved.transparent == false)
}

@Test func theDescriptionAfterTheFlagsIsKeptAndTheFlagsAreRebuilt() {
    #expect(mediaPrompt("a red fox") == "a red fox")
    #expect(mediaPrompt("/image --model grok-imagine --aspect 16:9 a red fox") == "a red fox")
    #expect(mediaPrompt("/video --ref /tmp/a.png --voice ara a pan") == "a pan")
    #expect(mediaPrompt("/image --transparent a cup") == "a cup")
    #expect(mediaPrompt("/image") == "")
    let choice = MediaChoice(kind: .image, model: "gpt-image", aspect: "4:3", duration: 0, resolution: "",
                             transparent: true, voice: nil)
    #expect(mediaDraft(choice, draft: "/image a cup") == "/image --model gpt-image --aspect 4:3 --transparent a cup")
    #expect(mediaDraft(choice, draft: "/") == "/image --model gpt-image --aspect 4:3 --transparent ")
    var video = choice
    video.kind = .video
    video.duration = 10
    video.resolution = "720p"
    video.voice = "ara"
    #expect(mediaDraft(video, draft: "") == "/video --model gpt-image --aspect 4:3 --duration 10 --resolution 720p --voice ara ")
}

@Test func aModelLineNamesWhoPaysAndAMissingAgentToo() {
    #expect(mediaDetail(model("m", [.image]), installed: true) == "maker · plan")
    #expect(mediaDetail(model("m", [.image]), installed: false).hasSuffix("not installed"))
}
