import AVFoundation
import CodyncKit
import Foundation

/// Records the microphone for a meeting and sends 20 s pieces to the computer, which transcribes
/// them when the recording stops. The phone keeps no audio after a piece is on its way.
@MainActor @Observable
public final class MeetingRecorder {
    public private(set) var started: MeetingStart?
    public private(set) var level: Float = 0
    public private(set) var sending = false
    public private(set) var transcribing = false
    public var failure: String?
    /// What the computer heard so far, for the recording pill.
    public private(set) var elapsed = 0

    @ObservationIgnored private lazy var engine = AVAudioEngine()
    @ObservationIgnored private var pending = Data()
    @ObservationIgnored private var lastAppend: Task<Void, Never>?
    @ObservationIgnored private var ticker: Task<Void, Never>?
    private static let pieceBytes = MicTap.bytesPerSecond * 20

    public init() {}

    public var isRecording: Bool { started != nil }

    /// Starts a recording on `store`'s computer, after asking for the microphone.
    public func start(store: BotStore, title: String) async {
        guard started == nil else { return }
        failure = nil
        guard await AVAudioApplication.requestRecordPermission() else {
            failure = "Allow microphone access in Settings to record a meeting."
            return
        }
        do {
            let start = try await store.startMeeting(title: title)
            started = start
            elapsed = 0
            pending = Data()
            guard Dictation.openMic(engine, bufferSize: 2048, tap: { format in
                MicTap.pcm16k(from: format) { [weak self] pcm, level in
                    Task { @MainActor in self?.heard(pcm, level: level, store: store) }
                }
            }) else {
                try? await store.cancelMeeting(start.id)
                started = nil
                failure = "The microphone isn't available."
                return
            }
            ticker = Task { [weak self] in
                while !Task.isCancelled {
                    try? await Task.sleep(for: .seconds(1))
                    guard let self, self.started != nil else { return }
                    self.elapsed += 1
                }
            }
        } catch {
            failure = error.localizedDescription
        }
    }

    private func heard(_ pcm: Data, level: Float, store: BotStore) {
        guard started != nil else { return }
        self.level = level
        pending.append(pcm)
        guard pending.count >= Self.pieceBytes else { return }
        let piece = pending
        pending = Data()
        send(piece, store: store)
    }

    private func send(_ piece: Data, store: BotStore) {
        guard let id = started?.id else { return }
        sending = true
        let previous = lastAppend
        lastAppend = Task { [weak self] in
            await previous?.value
            do {
                try await store.appendMeetingAudio(id, pcm: piece)
            } catch {
                await MainActor.run { self?.failure = error.localizedDescription }
            }
            await MainActor.run { self?.sending = false }
        }
    }

    /// Stops, sends the last piece and waits for the computer to write the transcript.
    public func finish(store: BotStore) async -> MeetingNote? {
        guard let started else { return nil }
        stopMicrophone()
        let rest = pending
        pending = Data()
        if !rest.isEmpty { send(rest, store: store) }
        transcribing = true
        defer { transcribing = false }
        await lastAppend?.value
        do {
            let note = try await store.finishMeeting(started.id)
            self.started = nil
            return note
        } catch {
            failure = error.localizedDescription
            return nil
        }
    }

    /// Stops and throws the recording away on the computer.
    public func cancel(store: BotStore) async {
        guard let started else { return }
        stopMicrophone()
        self.started = nil
        pending = Data()
        await lastAppend?.value
        _ = try? await store.cancelMeeting(started.id)
    }

    private func stopMicrophone() {
        ticker?.cancel()
        ticker = nil
        if engine.isRunning {
            engine.inputNode.removeTap(onBus: 0)
            engine.stop()
        }
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        level = 0
    }
}
