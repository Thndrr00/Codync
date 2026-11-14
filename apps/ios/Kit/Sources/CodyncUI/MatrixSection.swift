import CodyncKit
import SwiftUI

/// The Matrix connection of the computer: the homeserver, the bot's account, its bound rooms and the
/// people allowed in each. The session stays on the computer; the phone only sends the credentials
/// when connecting.
public struct MatrixSection: View {
    @Environment(BotStore.self) private var store
    @State private var status: MatrixStatus?
    @State private var editing = false
    @State private var homeserver = ""
    @State private var user = ""
    @State private var token = ""
    @State private var password = ""
    @State private var rooms: [RoomDraft] = []
    @State private var busy = false
    @State private var error: String?

    private struct RoomDraft: Identifiable, Equatable {
        let id = UUID()
        var roomId: String
        var botId: String
        var allowed: String
    }

    public init() {}

    private var choices: [Bot] {
        store.bots.values.filter { !$0.isGroup && $0.kind != "external" }.sorted { $0.name < $1.name }
    }

    private var fallbackBot: String { choices.first?.id ?? "" }

    public var body: some View {
        CardSection("Matrix", footer: "Make a Matrix account for the bot, invite it to the rooms, and paste its homeserver and credentials here. Leave the access token empty to sign in with the password once: the computer keeps the session.") {
            VStack(alignment: .leading, spacing: 6) {
                if let status, status.configured, !editing {
                    Text(line(status)).foregroundStyle(Palette.secondary)
                    ForEach(status.bindings) { binding in
                        Text("\(binding.roomId) · \(binding.allowedUsers.count) \(L10n.tr("people allowed"))")
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                    }
                    Button("Change Matrix settings") { start() }
                        .buttonStyle(.secondary)
                        .disabled(busy)
                    Button("Disconnect Matrix") {
                        act { status = try await store.disconnectMatrix() }
                    }
                    .buttonStyle(.secondary)
                    .disabled(busy)
                } else if status == nil, !editing {
                    Text(L10n.tr("Loading…")).foregroundStyle(Palette.secondary)
                } else {
                    if editing && rooms.isEmpty { start() }
                    Text(L10n.tr("Matrix homeserver")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($homeserver, prompt: "https://matrix.example.org")
                    Text(L10n.tr("Matrix user")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($user, prompt: "@bot:example.org")
                    Text(L10n.tr("Access token")).font(.caption).foregroundStyle(Palette.secondary)
                    secretField($token, prompt: L10n.tr("Leave empty to keep the saved session"))
                    Text(L10n.tr("Password")).font(.caption).foregroundStyle(Palette.secondary)
                    secretField($password, prompt: L10n.tr("Used once to sign in, never saved"))
                    ForEach($rooms) { $row in
                        Text(L10n.tr("Room id or alias")).font(.caption).foregroundStyle(Palette.secondary)
                        plainField($row.roomId, prompt: "!room:example.org")
                        ChoicePicker(selection: Binding(
                            get: { row.botId.isEmpty ? fallbackBot : row.botId },
                            set: { row.botId = $0 }
                        ), options: choices.map { (id: $0.id, label: $0.name) })
                        Text(L10n.tr("Allowed Matrix users")).font(.caption).foregroundStyle(Palette.secondary)
                        plainField($row.allowed, prompt: "@ana:example.org, @beto:example.org")
                        if rooms.count > 1 {
                            Button("Remove room") { rooms.removeAll { $0.id == row.id } }
                                .buttonStyle(.secondary)
                        }
                    }
                    HStack(spacing: 8) {
                        Button("Add room") { rooms.append(RoomDraft(roomId: "", botId: "", allowed: "")) }
                            .buttonStyle(.secondary)
                        Button(busy ? L10n.tr("Signing in…") : L10n.tr("Connect Matrix")) {
                            let bindings = clean
                            act {
                                status = try await store.setMatrix(homeserver: homeserver, user: user,
                                                                   accessToken: token, password: password,
                                                                   bindings: bindings)
                                editing = false
                                token = ""
                                password = ""
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

    private func line(_ status: MatrixStatus) -> String {
        let who = status.userId ?? ""
        let server = status.homeserver ?? ""
        return "\(L10n.tr(Channels.matrixStateText(status.state))) · \(who) · \(server) · \(status.bindings.count) \(L10n.tr("rooms bound"))"
    }

    private var clean: [MatrixBinding] {
        rooms.compactMap { row in
            let room = row.roomId.trimmed
            let allowed = row.allowed.split(whereSeparator: { $0.isWhitespace || $0 == "," }).map(String.init)
            guard !room.isEmpty || !allowed.isEmpty else { return nil }
            return MatrixBinding(roomId: room, botId: row.botId.isEmpty ? fallbackBot : row.botId,
                                 allowedUsers: allowed)
        }
    }

    private var ready: Bool {
        Channels.matrixHomeserverOk(homeserver) && Channels.matrixUserOk(user) && !clean.isEmpty
            && clean.allSatisfy { Channels.matrixRoomOk($0.roomId) && !$0.botId.isEmpty && !$0.allowedUsers.isEmpty
                && $0.allowedUsers.allSatisfy { Channels.matrixUserOk($0) } }
            && (!token.trimmed.isEmpty || !password.isEmpty || status?.configured == true)
    }

    private func start() {
        homeserver = status?.homeserver ?? ""
        user = status?.userId ?? ""
        token = ""
        password = ""
        rooms = (status?.bindings ?? []).map {
            RoomDraft(roomId: $0.roomId, botId: $0.botId, allowed: $0.allowedUsers.joined(separator: ", "))
        }
        if rooms.isEmpty { rooms = [RoomDraft(roomId: "", botId: "", allowed: "")] }
        editing = true
    }

    private func load() async {
        status = try? await store.matrixStatus()
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

    private func secretField(_ text: Binding<String>, prompt: String) -> some View {
        SecureField(prompt, text: text)
            .textFieldStyle(.plain)
            .plainTextInput()
            .padding(.horizontal, 14)
            .frame(height: 38)
            .background(Palette.bubbleUser, in: Capsule())
    }

    /// One change at a time; the computer's answer replaces what is shown.
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
