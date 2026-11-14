import CodyncKit
import SwiftUI

/// The local voice server the computer's OpenAI speech can go through: an OpenAI-compatible
/// server the person runs there (a sherpa-onnx or whisper server, a Parakeet or Cohere Transcribe
/// wrapper). Codync runs no model itself. The setting is the computer's, so the phone shows it and
/// can test it, like the voice keys; changing it happens on the computer.
struct LocalVoiceServerSection: View {
    @Environment(BotStore.self) private var store
    @State private var saved: VoiceServer?
    @State private var busy = false
    @State private var result: (ok: Bool, text: String)?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        CardSection(L10n.tr("Local voice server"),
                    footer: L10n.tr("Runs on this computer: Codync sends your dictation and spoken replies there, and no audio goes to OpenAI.")) {
            if let saved {
                if saved.custom {
                    ValueRow("Server URL", value: saved.effective)
                    if !saved.transcribeModel.isEmpty {
                        ValueRow("Dictation model", value: saved.transcribeModel)
                    }
                    if !saved.speechModel.isEmpty {
                        ValueRow("Speech model", value: saved.speechModel)
                    }
                } else {
                    Text(L10n.tr("No local voice server is set. Add the URL of the server on this computer."))
                        .foregroundStyle(Palette.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 10) {
                        Text(L10n.tr("Set on the computer. Add or change it in Codync on \(store.hostName)."))
                            .font(.caption)
                            .foregroundStyle(Palette.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                        Spacer(minLength: 8)
                        if busy {
                            Spinner()
                        } else {
                            Button(L10n.tr("Test server"), action: test)
                                .buttonStyle(.secondary)
                                .disabled(!saved.custom)
                        }
                    }
                    if let result {
                        Label(result.text, systemImage: result.ok ? "checkmark.circle.fill" : "exclamationmark.circle.fill")
                            .font(.footnote)
                            .foregroundStyle(result.ok ? Palette.secondary : Palette.danger)
                    }
                }
            } else {
                Spinner()
            }
        }
        .animation(Motion.reduced(Motion.fade, reduceMotion), value: result?.text)
        .animation(Motion.reduced(Motion.layout, reduceMotion), value: saved)
        .task { await load() }
    }

    private func load() async {
        guard let client = store.client else { return }
        saved = try? await client.voiceServer()
    }

    private func test() {
        guard !busy, let client = store.client else { return }
        busy = true
        result = nil
        Task {
            do {
                let answer = try await client.testVoiceServer()
                result = (answer.ok, answer.ok ? L10n.tr("Server works") : "\(L10n.tr("The server didn't answer")): \(answer.message)")
            } catch {
                result = (false, error.localizedDescription)
            }
            busy = false
        }
    }
}
