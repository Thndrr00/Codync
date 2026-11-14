import CodyncKit
import SwiftUI

/// Live voice with the computer's own Codex / ChatGPT sign-in (OpenBot 0.65). The sign-in lives in
/// Codex on the computer; Codync reads whether it can carry a realtime conversation and says why not
/// when it can't. No OpenAI key is needed for it.
struct CodexVoiceSection: View {
    @Environment(BotStore.self) private var store
    @State private var access: CodexVoiceAccess?
    @State private var busy = false

    var body: some View {
        CardSection(L10n.tr("Use my Codex sign-in"),
                    footer: L10n.tr("Live voice through Codex never asks for an OpenAI key: the call runs on your ChatGPT plan.")) {
            HStack(spacing: 8) {
                if busy {
                    Spinner()
                } else {
                    Image(systemName: access?.available == true ? "checkmark.circle.fill" : "info.circle")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(access?.available == true ? Palette.accent : Palette.tertiary)
                }
                Text(detail)
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 8)
                Button(L10n.tr("Check again"), action: check)
                    .buttonStyle(.secondary)
                    .disabled(busy || store.client == nil)
            }
        }
        .task { await load() }
    }

    private var detail: String {
        guard let access else { return L10n.tr("Codync is checking the sign-in.") }
        if !access.available { return access.reason ?? L10n.tr("Codex isn't signed in on this computer.") }
        guard let account = access.account, !account.isEmpty else { return L10n.tr("Signed in with ChatGPT in Codex.") }
        return L10n.tr("Signed in with ChatGPT as {account}.").replacingOccurrences(of: "{account}", with: account)
    }

    private func load() async {
        guard let client = store.client else { return }
        busy = true
        access = (try? await client.voiceStatus())?.codex
        busy = false
    }

    private func check() {
        Task { await load() }
    }
}
