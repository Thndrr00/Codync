import CodyncKit
import SwiftUI

extension BotStore {
    func serverSettings() async throws -> ServerSettings {
        try await ready().serverSettings()
    }

    func setServerSettings(_ settings: ServerSettings) async throws -> ServerSettings {
        try await ready().setServerSettings(settings)
    }

    /// Ends every remote control session; returns how many were running and whether a phone had
    /// taken control, so the sheet can say what it stopped.
    func stopAllRemoteControl() async throws -> (closed: Int, takeover: Bool) {
        let result = try await ready().screenStopAll()
        screen = result.screen
        return (result.closed, result.takeover)
    }
}

/// Settings that belong to this computer rather than to one bot: Personality (how its agents post
/// updates between steps), the Claude Code setup switch, and who may control it over remote desktop.
struct ServerSettingsSheet: View {
    @Environment(BotStore.self) private var store
    @State private var settings: ServerSettings?
    @State private var error: String?
    @State private var busy = false
    @State private var closed: Int?
    @State private var takeover = false
    @State private var confirmingStop = false

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Agents on this computer")
            ScrollView {
                if let settings {
                    VStack(alignment: .leading, spacing: 30) {
                        CardSection(
                            "Personality",
                            footer: "How the agents on this computer post updates between their steps. Natural writes a short note only when a finding changes the plan or before a long step, then an answer that starts with the verdict. Classic shows their own text as it comes."
                        ) {
                            ValueRow("Updates between steps") {
                                ChoicePicker(selection: Binding(
                                    get: { settings.responseStyle.rawValue },
                                    set: { save(ServerSettings(responseStyle: .init(rawValue: $0) ?? .natural,
                                                               claudeUserSetup: settings.claudeUserSetup,
                                                               remoteControl: settings.remoteControl)) }
                                ), options: [
                                    (id: "natural", label: L10n.tr("Natural")),
                                    (id: "classic", label: L10n.tr("Classic")),
                                ])
                            }
                        }

                        CardSection(
                            "Claude Code",
                            footer: "Off by default: your agents do not load your CLAUDE.md, skills, agents, hooks or plugins from ~/.claude, so each turn sends a shorter prompt. Your sign-in settings still apply, and the files of an agent's workspace still load."
                        ) {
                            SwitchRow(
                                "Use my Claude Code setup",
                                detail: "Give Claude agents your own Claude Code configuration. Turn this on if they should use your CLAUDE.md, skills or hooks.",
                                isOn: Binding(
                                    get: { settings.claudeUserSetup },
                                    set: { on in
                                        save(ServerSettings(responseStyle: settings.responseStyle,
                                                            claudeUserSetup: on,
                                                            remoteControl: settings.remoteControl))
                                    }
                                )
                            )
                        }

                        CardSection(
                            "Remote desktop",
                            footer: "Every device you paired may control this computer. Codync has one owner and no member roles yet, so \"Owners and admins\" reads as every device; \"Only this computer\" never lets a phone control it."
                        ) {
                            ValueRow("Who can control") {
                                ChoicePicker(selection: Binding(
                                    get: { settings.remoteControl.access.rawValue },
                                    set: { value in
                                        var next = settings.remoteControl
                                        next.access = .init(rawValue: value) ?? .everyone
                                        save(ServerSettings(responseStyle: settings.responseStyle,
                                                            claudeUserSetup: settings.claudeUserSetup,
                                                            remoteControl: next))
                                    }
                                ), options: [
                                    (id: "everyone", label: L10n.tr("Every device")),
                                    (id: "admins", label: L10n.tr("Owners and admins")),
                                    (id: "owner", label: L10n.tr("Only this computer")),
                                ])
                            }
                            SwitchRow(
                                "Ask on this computer",
                                detail: "Allow or deny each session here. A session nobody allows in 30 seconds does not start.",
                                isOn: Binding(
                                    get: { settings.remoteControl.askOnHost },
                                    set: { on in
                                        var next = settings.remoteControl
                                        next.askOnHost = on
                                        save(ServerSettings(responseStyle: settings.responseStyle,
                                                            claudeUserSetup: settings.claudeUserSetup,
                                                            remoteControl: next))
                                    }
                                )
                            )
                            ValueRow("Sessions", detail: sessionsDetail) {
                                Button(L10n.tr("Stop all")) { confirmingStop = true }
                                    .buttonStyle(.secondary)
                                    .disabled(busy)
                            }
                        }

                        if let error {
                            Text(L10n.tr(error)).font(.footnote).foregroundStyle(Palette.danger)
                        }
                    }
                    .padding(20)
                }
            }
        }
        .background(Palette.background)
        .task { settings = try? await store.serverSettings() }
        .codyncDialog(
            L10n.tr("Stop all remote control?"),
            isPresented: $confirmingStop,
            message: L10n.tr("Every screen session ends now, including one open on this phone. Remote screen stays on.")
        ) {
            [DialogAction(L10n.tr("Stop all"), destructive: true) { stopAll() }]
        }
    }

    /// What stopping every session did, or what the button will do.
    private var sessionsDetail: String {
        guard let closed else {
            return L10n.tr("Ends every remote control session now, without turning Remote screen off.")
        }
        let template = takeover
            ? L10n.tr("Stopped {count} session(s) and gave control back to the bots.")
            : L10n.tr("Stopped {count} session(s).")
        return template.replacingOccurrences(of: "{count}", with: String(closed))
    }

    private func save(_ next: ServerSettings) {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                settings = try await store.setServerSettings(next)
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func stopAll() {
        busy = true
        error = nil
        Task {
            do {
                let result = try await store.stopAllRemoteControl()
                closed = result.closed
                takeover = result.takeover
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
