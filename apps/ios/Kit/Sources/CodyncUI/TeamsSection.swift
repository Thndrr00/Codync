import CodyncKit
import SwiftUI

/// Microsoft Teams through an Azure Bot resource: the app id, its tenant, the client secret (or a
/// managed identity), the bot that answers and the people or tenants allowed. The secret stays on
/// the computer; the registration is the person's own Azure work.
public struct TeamsSection: View {
    @Environment(BotStore.self) private var store
    @State private var status: TeamsStatus?
    @State private var editing = false
    @State private var appId = ""
    @State private var tenantId = ""
    @State private var password = ""
    @State private var managed = false
    @State private var botId = ""
    @State private var users = ""
    @State private var tenants = ""
    @State private var memory = ""
    @State private var busy = false
    @State private var error: String?

    public init() {}

    private var choices: [Bot] {
        store.bots.values.filter { !$0.isGroup && $0.kind != "external" }.sorted { $0.name < $1.name }
    }

    private var fallbackBot: String { choices.first?.id ?? "" }
    private var userList: [String] { users.split(whereSeparator: { $0.isWhitespace || $0 == "," }).map(String.init) }
    private var tenantList: [String] { tenants.split(whereSeparator: { $0.isWhitespace || $0 == "," }).map(String.init) }

    public var body: some View {
        CardSection("Microsoft Teams", footer: "Create an Azure Bot resource, enable the Teams channel and point its messaging endpoint at this computer. Teams only reaches a public HTTPS address. The client secret stays on the computer.") {
            VStack(alignment: .leading, spacing: 6) {
                if let status, status.configured, !editing {
                    Text(line(status)).foregroundStyle(Palette.secondary)
                    Text(status.endpoint ?? "").font(.caption).foregroundStyle(Palette.tertiary)
                    Button("Change Teams settings") { start() }
                        .buttonStyle(.secondary)
                        .disabled(busy)
                    Button("Disconnect Teams") {
                        act { status = try await store.disconnectTeams() }
                    }
                    .buttonStyle(.secondary)
                    .disabled(busy)
                } else if status == nil, !editing {
                    Text(L10n.tr("Loading…")).foregroundStyle(Palette.secondary)
                } else {
                    Text("\(L10n.tr("Microsoft Teams")) — \(L10n.tr(Channels.teamsStateText(status?.state ?? "notConfigured")))")
                        .foregroundStyle(Palette.secondary)
                    if let registration = status?.registration {
                        Text(registration).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    Text(L10n.tr("Microsoft app id")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($appId, prompt: "00000000-0000-0000-0000-000000000000")
                    Text(L10n.tr("Directory tenant")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($tenantId, prompt: "contoso.onmicrosoft.com, or empty")
                    ChoicePicker(selection: Binding(
                        get: { managed ? "managed" : "secret" },
                        set: { managed = $0 == "managed" }
                    ), options: [
                        (id: "secret", label: L10n.tr("App client secret")),
                        (id: "managed", label: L10n.tr("Managed identity")),
                    ])
                    if !managed {
                        Text(L10n.tr("Client secret")).font(.caption).foregroundStyle(Palette.secondary)
                        secretField($password, prompt: status?.configured == true ? L10n.tr("Leave empty to keep the saved secret") : L10n.tr("The app registration secret"))
                    }
                    ChoicePicker(selection: Binding(
                        get: { botId.isEmpty ? fallbackBot : botId },
                        set: { botId = $0 }
                    ), options: choices.map { (id: $0.id, label: $0.name) })
                    Text(L10n.tr("People allowed")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($users, prompt: "ana@example.com, or an AAD object id, or *")
                    Text(L10n.tr("Tenants allowed")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($tenants, prompt: "a tenant id, or *")
                    Text(L10n.tr("What the bot knows about Teams")).font(.caption).foregroundStyle(Palette.secondary)
                    plainField($memory, prompt: L10n.tr("Given to the bot before each Teams message"))
                    HStack(spacing: 8) {
                        Button(busy ? L10n.tr("Connecting…") : L10n.tr("Connect Teams")) {
                            act {
                                status = try await store.setTeams(
                                    appId: appId, tenantId: tenantId, password: password, managedIdentity: managed,
                                    botId: botId.isEmpty ? fallbackBot : botId, allowedUsers: userList,
                                    allowedTenants: tenantList, memory: memory, icon: "")
                                editing = false
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

    private func line(_ status: TeamsStatus) -> String {
        let bot = choices.first { $0.id == (status.botId ?? "") }?.name ?? status.botId ?? ""
        return "\(L10n.tr(Channels.teamsStateText(status.state))) — \(status.appId ?? "") — \(bot) — \(status.allowedUsers.count) \(L10n.tr("people allowed"))"
    }

    private var ready: Bool {
        appId.trimmingCharacters(in: .whitespacesAndNewlines).wholeMatch(of: /[0-9a-fA-F-]{36}/) != nil
            && (managed || !password.isEmpty || status?.configured == true)
            && !(botId.isEmpty && fallbackBot.isEmpty)
            && !(userList.isEmpty && tenantList.isEmpty)
    }

    private func start() {
        appId = status?.appId ?? ""
        tenantId = status?.tenantId ?? ""
        password = ""
        managed = status?.managedIdentity ?? false
        botId = status?.botId ?? ""
        users = (status?.allowedUsers ?? []).joined(separator: ", ")
        tenants = (status?.allowedTenants ?? []).joined(separator: ", ")
        memory = status?.memory ?? ""
        editing = true
    }

    private func load() async {
        status = try? await store.teamsStatus()
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
