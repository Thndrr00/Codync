import CodyncKit
import SwiftUI

/// The Signal channel: a signal-cli daemon on the computer, the number it has linked, and the chats
/// allowed to write to a bot. The daemon owns the account; the phone only sends the setup.
public struct SignalSection: View {
    @Environment(BotStore.self) private var store
    @State private var status: SignalStatus?
    @State private var editing = false
    @State private var number = ""
    @State private var daemon = "http://127.0.0.1:7583"
    @State private var chats: [ChatDraft] = []
    @State private var busy = false
    @State private var error: String?

    private struct ChatDraft: Identifiable, Equatable {
        let id = UUID()
        var chat: String
        var botId: String
        var allowed: String
    }

    public init() {}

    private var choices: [Bot] {
        store.bots.values.filter { !$0.isGroup && $0.kind != "external" }.sorted { $0.name < $1.name }
    }

    private var fallbackBot: String { choices.first?.id ?? "" }

    public var body: some View {
        CardSection("Signal", footer: "Link a number to signal-cli on the computer, run its daemon, then save the number and the daemon here. The number and keys stay on the computer.") {
            VStack(alignment: .leading, spacing: 6) {
                if let status, status.configured, !editing {
                    Text(line(status)).foregroundStyle(Palette.secondary)
                    ForEach(status.bindings) { binding in
                        Text(binding.chat + " — " + binding.allowedUsers.count.description + " " + L10n.tr("people allowed"))
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                    }
                    if let setup = status.setup {
                        Text(setup).font(.footnote).foregroundStyle(Palette.text)
                    }
                    Button("Change Signal settings") { start() }
                        .buttonStyle(.secondary)
                        .disabled(busy)
                    Button("Disconnect Signal") {
                        act { status = try await store.disconnectSignal() }
                    }
                    .buttonStyle(.secondary)
                    .disabled(busy)
                } else if status == nil, !editing {
                    Text(L10n.tr("Loading…")).foregroundStyle(Palette.secondary)
                } else {
                    if editing && chats.isEmpty { start() }
                    if let setup = status?.setup {
                        Text(setup).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    Text(L10n.tr("Signal number")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($number, prompt: "+15551234567")
                    Text(L10n.tr("signal-cli daemon")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($daemon, prompt: "http://127.0.0.1:7583")
                    ForEach($chats) { $row in
                        Text(L10n.tr("Phone number or group")).font(.caption).foregroundStyle(Palette.secondary)
                        plainField($row.chat, prompt: "+15551234567 or group:<id>")
                        ChoicePicker(selection: Binding(
                            get: { row.botId.isEmpty ? fallbackBot : row.botId },
                            set: { row.botId = $0 }
                        ), options: choices.map { (id: $0.id, label: $0.name) })
                        Text(L10n.tr("Allowed Signal numbers")).font(.caption).foregroundStyle(Palette.secondary)
                        plainField($row.allowed, prompt: "+15557654321, or a UUID")
                        if chats.count > 1 {
                            Button("Remove chat") { chats.removeAll { $0.id == row.id } }
                                .buttonStyle(.secondary)
                        }
                    }
                    HStack(spacing: 8) {
                        Button("Add chat") { chats.append(ChatDraft(chat: "", botId: "", allowed: "")) }
                            .buttonStyle(.secondary)
                        Button(busy ? L10n.tr("Connecting…") : L10n.tr("Connect Signal")) {
                            let bindings = clean
                            act {
                                status = try await store.setSignal(number: number, daemon: daemon, bindings: bindings)
                                editing = false
                            }
                        }
                        .buttonStyle(.secondary)
                        .disabled(busy || !ready)
                        if status?.configured == true {
                            Button("Cancel") { editing = false }
                                .buttonStyle(.secondary)
                                .disabled(busy)
                        }
                    }
                }
                if let error {
                    Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger)
                }
            }
            .padding(.vertical, 2)
        }
        .task { await load() }
    }

    private func line(_ status: SignalStatus) -> String {
        "\(L10n.tr(Channels.signalStateText(status.state))) — \(status.number ?? "") — \(status.bindings.count) \(L10n.tr("chats bound"))"
    }

    private var clean: [SignalBinding] {
        chats.compactMap { row in
            let chat = row.chat.trimmed
            let allowed = row.allowed.split(whereSeparator: { $0.isWhitespace || $0 == "," }).map(String.init)
            guard !chat.isEmpty || !allowed.isEmpty else { return nil }
            return SignalBinding(chat: chat, botId: row.botId.isEmpty ? fallbackBot : row.botId, allowedUsers: allowed)
        }
    }

    private var ready: Bool {
        Channels.signalNumberOk(number) && !daemon.trimmed.isEmpty && !clean.isEmpty
            && clean.allSatisfy { Channels.signalChatOk($0.chat) && !$0.botId.isEmpty && !$0.allowedUsers.isEmpty
                && $0.allowedUsers.allSatisfy { Channels.signalPersonOk($0) } }
    }

    private func start() {
        number = status?.number ?? ""
        daemon = status?.daemon ?? "http://127.0.0.1:7583"
        chats = (status?.bindings ?? []).map {
            ChatDraft(chat: $0.chat, botId: $0.botId, allowed: $0.allowedUsers.joined(separator: ", "))
        }
        if chats.isEmpty { chats = [ChatDraft(chat: "", botId: "", allowed: "")] }
        editing = true
    }

    private func load() async {
        status = try? await store.signalStatus()
    }

    private func plainField(_ text: Binding<String>, prompt: String) -> some View {
        TextField(prompt, text: text)
            .textFieldStyle(.plain)
            .plainTextInput()
            .autocorrectionDisabled()
            .textInputAutocapitalization(.never)
            .padding(.horizontal, 14)
            .frame(height: 38)
            .background(Palette.bubbleUser, in: Capsule())
    }

    private func act(_ work: @escaping @MainActor () async throws -> Void) {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                try await work()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}

private extension String {
    var trimmed: String { trimmingCharacters(in: .whitespacesAndNewlines) }
}
