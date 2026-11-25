import Testing
@testable import CodyncUI

@Test func spokenTextDropsMarkdown() {
    let reply = """
    ## Done

    Fixed **the login page** in `LoginView.swift`, see [the PR](https://x.y/1).

    ```swift
    let a = 1
    ```

    - kept `user_id` as is
    | File | Lines |
    |------|-------|
    | a.swift | 3 |
    """
    #expect(SpokenText.from(reply) == """
    Done
    Fixed the login page in LoginView.swift, see the PR.
    kept user_id as is
    File, Lines
    a.swift, 3
    """)
}

@Test func spokenTextDropsBulletsTaskBoxesQuotesAndSetextRules() {
    let reply = """
    ## Plan
    > - outer bullet
    > > • nested bullet
    1. - [ ] first task
       * [x] nested task
    + keep C++, x*2 and #tag intact
    Heading
    ===
    Plain prose with a quote: "ship it" and a dash - inside.
    """
    #expect(SpokenText.cleanSpokenText(reply) == """
    Plan
    outer bullet
    nested bullet
    first task
    nested task
    keep C++, x*2 and #tag intact
    Heading

    Plain prose with a quote: "ship it" and a dash - inside.
    """)
    #expect(SpokenText.from("- **Bold** and [link](https://x.y/1)\n===\n\nMore.") == "Bold and link\nMore.")
}

@Test func aRepeatedProviderUtteranceIsPostedOnceInsideTheWindow() {
    let at = Date(timeIntervalSince1970: 10_000)
    let posted = SpokenText.Utterance(text: "ship it", at: at)
    #expect(SpokenText.isRepeatUtterance("ship it", posted, windowSeconds: 2, now: at.addingTimeInterval(1)))
    #expect(SpokenText.isRepeatUtterance("  ship it  ", posted, windowSeconds: 2, now: at.addingTimeInterval(1)))
    #expect(!SpokenText.isRepeatUtterance("ship it", posted, windowSeconds: 2, now: at.addingTimeInterval(2.001)))
    #expect(!SpokenText.isRepeatUtterance("ship it again", posted, windowSeconds: 2, now: at.addingTimeInterval(1)))
    #expect(!SpokenText.isRepeatUtterance("ship it", nil, windowSeconds: 2, now: at.addingTimeInterval(1)))
    #expect(!SpokenText.isRepeatUtterance("", SpokenText.Utterance(text: "", at: at), windowSeconds: 2, now: at.addingTimeInterval(1)))
    #expect(!SpokenText.isRepeatUtterance("ship it", SpokenText.Utterance(text: "ship it", at: at.addingTimeInterval(1.5)), windowSeconds: 2, now: at.addingTimeInterval(1)))
}

@Test func onlyANewExplicitUserMessageInterruptsReadingAloud() {
    #expect(SpokenText.shouldInterruptSpeech(source: .user, isUserMainMessage: true))
    for source in [SpokenText.Source.channel, .thread, .selfTurn, .notice] {
        #expect(!SpokenText.shouldInterruptSpeech(source: source, isUserMainMessage: true))
        #expect(!SpokenText.shouldInterruptSpeech(source: source, isUserMainMessage: false))
    }
    #expect(!SpokenText.shouldInterruptSpeech(source: .user, isUserMainMessage: false))
}
