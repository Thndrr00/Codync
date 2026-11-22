import Foundation
import Testing
@testable import CodyncKit
@testable import CodyncUI

/// A long recording is split at quiet points into parts no longer than 90 s, and the parts cover
/// every byte in order. An even loudness cuts at the limit; a quiet window moves the cut back to it.
@Test func dictationPartsCoverTheRecordingAndRespectTheLimit() {
    let part = DictationSplit.partBytes
    let loud = { (_: Range<Int>) in 1.0 }
    #expect(DictationSplit.ranges(of: part / 2, loudness: loud) == [0..<(part / 2)])
    #expect(DictationSplit.ranges(of: part * 2, loudness: loud) == [0..<part, part..<part * 2])
    #expect(DictationSplit.ranges(of: 0, loudness: loud).isEmpty)
    let ranges = DictationSplit.ranges(of: part * 2 + 1, loudness: loud)
    #expect(ranges.count == 2)
    #expect(ranges.first?.lowerBound == 0)
    #expect(ranges.last?.upperBound == part * 2 + 1)
    #expect(ranges.allSatisfy { $0.count <= part })
}

@Test func aLongDictationBreaksAtTheQuietestNearbyPoint() {
    let part = DictationSplit.partBytes
    let second = MicTap.bytesPerSecond
    let count = part * 2 - second
    let quiet = part - second
    let ranges = DictationSplit.ranges(of: count) { range in
        range.lowerBound == quiet ? 0 : 1
    }
    #expect(ranges.count == 2)
    #expect(ranges[0] == 0..<quiet)
    #expect(ranges[1] == quiet..<count)
    // A recording can run for ten minutes; that is about four 90-second parts.
    #expect(DictationSplit.maxBytes == second * 600)
}

@Test func dictationLoudnessReadsPcmAmplitude() {
    var pcm = Data(repeating: 0, count: 20)
    #expect(DictationSplit.level(of: pcm) == 0)
    for i in 0..<10 { pcm.replaceSubrange(i * 2..<(i * 2 + 2), with: withUnsafeBytes(of: Int16.max) { Data($0) }) }
    #expect(DictationSplit.level(of: pcm) > 0.6)
}

@Test @MainActor func dictatedWordsGoAfterTheBox() {
    #expect(Dictation.join("", "hello") == "hello")
    #expect(Dictation.join("Fix the bug", "in the parser") == "Fix the bug in the parser")
    #expect(Dictation.join("Fix the bug\n", "now") == "Fix the bug\nnow")
    #expect(Dictation.join("Fix the bug", "") == "Fix the bug")
}

/// A provider's network call that the test answers when it chooses.
@MainActor private final class Pending<Value: Sendable> {
    private var waiting: CheckedContinuation<Value, any Error>?
    private(set) var calls = 0
    /// Calls that have had their answer.
    private(set) var returned = 0

    func call() async throws -> Value {
        calls += 1
        defer { returned += 1 }
        return try await withCheckedThrowingContinuation { waiting = $0 }
    }

    /// Whether the call is waiting for its answer (2 s at most).
    func asked() async -> Bool {
        await until { self.waiting != nil }
    }

    func answer(_ value: Value) {
        waiting?.resume(returning: value)
        waiting = nil
    }

    func fail(_ error: any Error) {
        waiting?.resume(throwing: error)
        waiting = nil
    }
}

/// The phone's microphone, hearing what the test plays it.
@MainActor private final class Mic {
    /// Where the open microphone's audio goes; nil until it's opened.
    var heard: (@Sendable (Data, Float) -> Void)?
}

private struct Offline: LocalizedError {
    var errorDescription: String? { "The computer is offline." }
}

/// Dictation with OpenAI picked, the computer's answers and the microphone in the test's hands, and a store to dictate through.
@MainActor private func dictating(hasKey: @escaping @MainActor () async throws -> Bool = { true },
                                  transcript: Pending<String> = Pending(), mic: Mic = Mic(),
                                  _ body: (Dictation, BotStore) async -> Void) async {
    let suite = "DictationTests.\(UUID().uuidString)"
    let storage = SharedStore.Context(accountID: UUID().uuidString, suite: suite)
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let store = BotStore(computer: Computer(id: "dictation", name: "Mac", signKey: ""), storage: storage) { _ in
        throw HostError.unreachable
    }
    defer { store.retire() }
    let service = DictationService(picked: { .openAI },
                                   hasKey: { _, _ in try await hasKey() },
                                   allowMic: { true },
                                   openMic: { _, heard in
                                       mic.heard = heard
                                       return true
                                   },
                                   transcribe: { _, _, _ in try await transcript.call() })
    await body(Dictation(service: service), store)
}

private extension Dictation {
    /// Records `bytes` of what `mic` hears and taps the pill's stop; the transcript changes the box through `edit`.
    func dictate(_ bytes: Int, from mic: Mic, on store: BotStore, edit: @escaping ((String) -> String) -> Void) async {
        await start(after: "", model: store, edit: edit)
        mic.heard?(Data(count: bytes), 0.5)
        #expect(await until { self.level > 0 })
        finish()
    }
}

/// The pill's stop has the recording transcribed, and the words go after what's in the box when
/// they arrive, words typed while waiting included.
@MainActor @Test func aRecordingIsTranscribedAfterWhatTheBoxHoldsThen() async {
    let transcript = Pending<String>(), mic = Mic()
    await dictating(transcript: transcript, mic: mic) { dictation, store in
        var box = "Fix the bug"
        await dictation.dictate(Dictation.minBytes, from: mic, on: store) { box = $0(box) }
        #expect(dictation.transcribing && !dictation.listening)
        #expect(await transcript.asked())
        box += " now"
        transcript.answer("in the parser")
        #expect(await until { !dictation.transcribing })
        #expect(box == "Fix the bug now in the parser")
        #expect(dictation.failure == nil)
    }
}

/// A stop (a send, another chat) while the transcript is on its way drops it when it comes.
@MainActor @Test func aTranscriptArrivingAfterAStopIsDropped() async {
    let transcript = Pending<String>(), mic = Mic()
    await dictating(transcript: transcript, mic: mic) { dictation, store in
        var box = "Fix the bug"
        await dictation.dictate(Dictation.minBytes, from: mic, on: store) { box = $0(box) }
        #expect(await transcript.asked())
        dictation.stop()
        #expect(!dictation.transcribing)
        transcript.answer("too late")
        #expect(await until { transcript.returned == 1 })
        #expect(box == "Fix the bug")
        #expect(dictation.failure == nil && !dictation.transcribing)
    }
}

/// A failed transcription says why and leaves the box as it was.
@MainActor @Test func aFailedTranscriptionSaysWhy() async {
    let transcript = Pending<String>(), mic = Mic()
    await dictating(transcript: transcript, mic: mic) { dictation, store in
        var box = "Fix the bug"
        await dictation.dictate(Dictation.minBytes, from: mic, on: store) { box = $0(box) }
        #expect(await transcript.asked())
        transcript.fail(Offline())
        #expect(await until { !dictation.transcribing })
        #expect(dictation.failure == "The computer is offline.")
        #expect(box == "Fix the bug")
    }
}

/// Under half a second is a tap of the mic: nothing goes out on the user's key.
@MainActor @Test func aTapOfTheMicSendsNothing() async {
    let transcript = Pending<String>(), mic = Mic()
    await dictating(transcript: transcript, mic: mic) { dictation, store in
        await dictation.dictate(Dictation.minBytes - 1, from: mic, on: store) { _ in }
        #expect(!dictation.transcribing)
        #expect(transcript.calls == 0)
    }
}

/// What the microphone heard before a stop never joins the next recording.
@MainActor @Test func audioHeardBeforeAStopIsDropped() async {
    let mic = Mic()
    await dictating(mic: mic) { dictation, store in
        await dictation.start(after: "", model: store) { _ in }
        let earlier = mic.heard
        dictation.stop()
        await dictation.start(after: "", model: store) { _ in }
        earlier?(Data(count: Dictation.minBytes), 0.9)
        mic.heard?(Data(), 0.5)
        #expect(await until { dictation.level == 0.5 })
        dictation.finish()
        #expect(!dictation.transcribing)
    }
}

/// Without the provider's key on the computer it opens settings, and never opens the microphone.
@MainActor @Test func dictatingWithoutAKeyRequestsSettings() async {
    let key = Pending<Bool>(), mic = Mic()
    await dictating(hasKey: { try await key.call() }, mic: mic) { dictation, store in
        let starting = Task { await dictation.start(after: "", model: store) { _ in } }
        #expect(await key.asked())
        key.answer(false)
        await starting.value
        #expect(dictation.failure == nil && dictation.needsKey)
        #expect(!dictation.listening && mic.heard == nil)
    }
}

/// With the computer out of reach the key check says why, and no recording starts.
@MainActor @Test func anUnreachableComputerSaysWhy() async {
    let key = Pending<Bool>(), mic = Mic()
    await dictating(hasKey: { try await key.call() }, mic: mic) { dictation, store in
        let starting = Task { await dictation.start(after: "", model: store) { _ in } }
        #expect(await key.asked())
        key.fail(Offline())
        await starting.value
        #expect(dictation.failure == "The computer is offline.")
        #expect(!dictation.listening && mic.heard == nil)
    }
}

/// A stop while the key check is on its way wins, whatever the check says: no recording starts and nothing is reported.
@MainActor @Test(arguments: [true, false]) func aStopWhileTheKeyIsCheckedWins(checkAnswers: Bool) async {
    let key = Pending<Bool>(), mic = Mic()
    await dictating(hasKey: { try await key.call() }, mic: mic) { dictation, store in
        let starting = Task { await dictation.start(after: "", model: store) { _ in } }
        #expect(await key.asked())
        dictation.stop()
        if checkAnswers { key.answer(true) } else { key.fail(Offline()) }
        await starting.value
        #expect(!dictation.listening && mic.heard == nil && dictation.failure == nil)
    }
}
