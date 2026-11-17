import CodyncUI
import Foundation
import SwiftUI

/// Meeting notes on the phone: record the microphone, let the computer write the transcript, and
/// ask a bot for the minutes. The audio never stays on the phone.
struct MeetingsSectionView: View {
    let store: BotStore

    @State private var notes: [MeetingSummary] = []
    @State private var title = ""
    @State private var recorder = MeetingRecorder()
    @State private var open: MeetingNote?
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        CardSection("Meetings", footer: "Record the microphone here. The computer writes it down and a bot can draft the minutes; keep Codync open while it records.") {
            if let started = recorder.started {
                HStack(spacing: 8) {
                    Image(systemName: "waveform")
                        .foregroundStyle(Palette.accent)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(started.title).foregroundStyle(Palette.text)
                        Text(recorder.sending ? "Sending the last piece…" : time(recorder.elapsed))
                            .font(.caption)
                            .foregroundStyle(Palette.secondary)
                    }
                    Spacer(minLength: 0)
                    Button(recorder.transcribing ? "Writing it down…" : "Stop") { stop() }
                        .buttonStyle(.secondary)
                        .disabled(recorder.transcribing)
                }
                Button("Discard this recording", role: .destructive) { discard() }
                    .buttonStyle(.plain)
                    .font(.caption)
                    .foregroundStyle(Palette.danger)
            } else {
                TextField("Meeting title", text: $title).plainTextInput()
                HStack {
                    Spacer()
                    Button("Record a meeting") { start() }
                        .buttonStyle(.secondary)
                        .disabled(busy)
                }
            }

            ForEach(notes) { note in
                HStack(spacing: 8) {
                    Button { show(note) } label: {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(note.title).foregroundStyle(Palette.text)
                            Text("\(when(note.startedAt)) · \(note.chars.formatted()) characters\(note.hasMinutes ? " · Minutes" : "")")
                                .font(.caption)
                                .foregroundStyle(Palette.secondary)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    Spacer(minLength: 0)
                    if !note.hasMinutes {
                        Button("Minutes") { minutes(note) }
                            .buttonStyle(.secondary)
                            .disabled(busy)
                    }
                }
                .contextActions { [MenuItem("Delete", icon: "trash", destructive: true, divider: true) { remove(note) }] }
            }
            if notes.isEmpty, !busy {
                Text("No meeting notes yet.").font(.caption).foregroundStyle(Palette.secondary)
            }

            if let open, notes.contains(where: { $0.id == open.id }) {
                VStack(alignment: .leading, spacing: 6) {
                    if let minutes = open.minutes, !minutes.isEmpty {
                        Text("Minutes").font(.caption).fontWeight(.semibold).foregroundStyle(Palette.secondary)
                        Text(minutes).font(.callout).foregroundStyle(Palette.text)
                    }
                    Text("Transcript").font(.caption).fontWeight(.semibold).foregroundStyle(Palette.secondary)
                    Text(open.transcript).font(.callout).foregroundStyle(Palette.text)
                }
                .padding(10)
                .background(Palette.surface, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            }

            if let error {
                Text(error).font(.caption).foregroundStyle(Palette.danger)
            }
        }
        .task { await load() }
        .onDisappear { Task { await recorder.cancel(store: store) } }
    }

    private func load() async {
        do {
            notes = try await store.meetingNotes()
            error = nil
        } catch { error = message(error) }
    }

    private func start() {
        busy = true
        Task {
            await recorder.start(store: store, title: title.trimmingCharacters(in: .whitespacesAndNewlines))
            if recorder.started != nil { title = "" }
            if let failure = recorder.failure { error = failure }
            busy = false
        }
    }

    private func stop() {
        Task {
            if let note = await recorder.finish(store: store) {
                open = note
                await load()
            }
            if let failure = recorder.failure { error = failure }
        }
    }

    private func discard() {
        Task {
            await recorder.cancel(store: store)
            open = nil
        }
    }

    private func show(_ note: MeetingSummary) {
        if open?.id == note.id {
            open = nil
            return
        }
        Task {
            do {
                open = try await store.meetingNote(note.id)
                error = nil
            } catch { error = message(error) }
        }
    }

    private func minutes(_ note: MeetingSummary) {
        busy = true
        Task {
            do {
                open = try await store.meetingMinutes(note.id)
                await load()
            } catch { error = message(error) }
            busy = false
        }
    }

    private func remove(_ note: MeetingSummary) {
        Task {
            do {
                _ = try await store.deleteMeetingNote(note.id)
                if open?.id == note.id { open = nil }
                await load()
            } catch { error = message(error) }
        }
    }

    private func when(_ ms: Int) -> String {
        Date(timeIntervalSince1970: Double(ms) / 1000)
            .formatted(.dateTime.day().month(.abbreviated).hour().minute())
    }

    private func time(_ seconds: Int) -> String {
        String(format: "%d:%02d", seconds / 60, seconds % 60)
    }

    private func message(_ error: Error) -> String {
        (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
}
