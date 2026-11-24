import SwiftUI
import CodyncKit

/// The agentes.chat behaviors in one sheet: setup, presence, stretch and approval cards, and the vault.
/// The view holds no I/O: it calls the typed HostClient bridge and renders what the host answers.

public struct AgentesChatView: View {
    private let host: HostClient
    private let botId: String
    @State private var setup: SetupChecklist?
    @State private var presence: PresenceStatus?
    @State private var status = ""
    @State private var card: StretchCard?
    @State private var approval: ApprovalRequest?
    @State private var secret = ""
    @State private var masked = ""
    @State private var error = ""

    public init(host: HostClient, botId: String) {
        self.host = host
        self.botId = botId
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                ModalHeader(title: "Agentes.chat")
                if !error.isEmpty { Text(error).foregroundStyle(.red) }

                section("Setup") {
                    if let setup {
                        ForEach(setup.steps) { step in
                            Label(step.label, systemImage: step.done ? "checkmark.circle.fill" : "circle")
                        }
                        Text("\(setup.done)/\(setup.total) done").foregroundStyle(.secondary)
                    } else { ProgressView() }
                }

                section("Presence") {
                    Text(presence?.text ?? "No status set").foregroundStyle(.secondary)
                    HStack {
                        TextField("Back at 3", text: $status)
                        Button("Set") { Task { await saveStatus() } }.buttonStyle(SecondaryButtonStyle())
                    }
                }

                section("Cards") {
                    Button("Preview cards") { Task { await previewCards() } }.buttonStyle(SecondaryButtonStyle())
                    if let card { Text("\(card.type) · \(card.bodyCode)").foregroundStyle(.secondary) }
                    if let approval {
                        Text(approval.reason)
                        if approval.status == "pending" {
                            HStack {
                                ForEach(approval.options) { option in
                                    Button(option.label) { Task { await decide(option.id) } }
                                        .buttonStyle(SecondaryButtonStyle())
                                }
                            }
                        } else { Text("Answered: \(approval.selected ?? "")").foregroundStyle(.secondary) }
                    }
                }

                section("Vault") {
                    SecureField("secret", text: $secret)
                    Button("Mask") { Task { await mask() } }.buttonStyle(SecondaryButtonStyle())
                    if !masked.isEmpty { Text(masked).foregroundStyle(.secondary) }
                }
            }
            .padding()
        }
        .task { await load() }
    }

    private func section<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.headline)
            content()
        }
    }

    private func load() async {
        do {
            async let s = host.setup(botId: botId)
            async let p = host.presence(botId: botId)
            setup = try await s
            presence = try await p
        } catch { error = String(describing: error) }
    }

    private func saveStatus() async {
        do { presence = try await host.setPresence(botId: botId, status: status); status = "" }
        catch { error = String(describing: error) }
    }

    private func previewCards() async {
        do {
            card = try await host.stretchCard(agent: "the bot", reason: "runs")
            approval = try await host.approvalCard(agent: "the bot", reason: "needs to send an email", requestId: "request-1")
        } catch { error = String(describing: error) }
    }

    private func decide(_ option: String) async {
        guard let approval else { return }
        do { self.approval = try await host.decideApproval(approval, option: option) }
        catch { error = String(describing: error) }
    }

    private func mask() async {
        do { masked = try await host.vaultMask(name: "openai", secret: secret).masked }
        catch { error = String(describing: error) }
    }
}
