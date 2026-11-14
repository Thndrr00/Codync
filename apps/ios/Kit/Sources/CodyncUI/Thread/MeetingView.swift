import AVFoundation
import CodyncKit
import SwiftUI
import UIKit

/// Plays a meeting turn's spoken audio: the host returns base64 in a container AVAudioPlayer reads.
@MainActor
final class MeetingAudio: NSObject, ObservableObject, AVAudioPlayerDelegate {
    @Published private(set) var playing: String?
    private var player: AVAudioPlayer?

    func play(_ turn: MeetingTurn) {
        guard let encoded = turn.audio, let data = Data(base64Encoded: encoded) else { return }
        stop()
        let suffix = (turn.audioFormat ?? "mp3").lowercased()
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("meeting-\(turn.botId)-\(UUID().uuidString).\(suffix)")
        do {
            try data.write(to: url)
            let player = try AVAudioPlayer(contentsOf: url)
            player.delegate = self
            self.player = player
            playing = turn.botId
            player.play()
        } catch {
            playing = nil
        }
    }

    func stop() {
        player?.stop()
        player = nil
        playing = nil
    }

    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        Task { @MainActor in self.stop() }
    }
}

/// A short roundtable of a group's members on one topic, with the draft minutes and each turn
/// spoken when the computer had a voice provider.
public struct MeetingView: View {
    let groupId: String
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var topic = ""
    @State private var meeting: TeamMeeting?
    @State private var running = false
    @State private var error: String?
    @StateObject private var audio = MeetingAudio()

    public init(groupId: String) { self.groupId = groupId }

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Team meeting") {
                if running { Spinner() }
            }
            CardForm {
                CardSection("Topic", footer: "The group's members answer in turn, in throwaway sessions. Nothing lands in the chat; the minutes are a draft you can copy.") {
                    TextField("What should the team think through?", text: $topic, axis: .vertical)
                        .plainTextInput()
                        .padding(10)
                        .background(Palette.surface, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    HStack {
                        Spacer()
                        Button(running ? "Thinking…" : "Start meeting") { start() }
                            .buttonStyle(.primary)
                            .disabled(running || topic.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }
                }
                if let meeting {
                    CardSection("Minutes") {
                        Text(meeting.minutes).textSelection(.enabled).foregroundStyle(Palette.text)
                        HStack {
                            Spacer()
                            Button("Copy minutes") { UIPasteboard.general.string = meeting.minutes }
                                .buttonStyle(.secondary)
                        }
                    }
                    ForEach(meeting.turns) { turn in
                        CardSection(turn.name) {
                            Text(turn.text).foregroundStyle(Palette.text)
                            if turn.audio != nil {
                                HStack {
                                    Spacer()
                                    Button(audio.playing == turn.id ? "Stop" : "Read aloud") {
                                        if audio.playing == turn.id { audio.stop() } else { audio.play(turn) }
                                    }
                                    .buttonStyle(.secondary)
                                }
                            }
                        }
                    }
                }
                if let error {
                    Text(error).font(.footnote).foregroundStyle(Palette.danger)
                }
            }
        }
        .onDisappear { audio.stop() }
    }

    private func start() {
        let question = topic.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !question.isEmpty else { return }
        running = true
        error = nil
        Task {
            defer { running = false }
            do {
                meeting = try await model.teamMeeting(groupId: groupId, topic: question, voice: true)
            } catch {
                self.error = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
            }
        }
    }
}
