import CodyncKit
import SwiftUI

/// The computer-wide ceiling on local mini-apps: a safeguard on this computer, not a plan. Only a new
/// app takes a slot, and a bot may always update an app it already made.
/// The computer is the `BotStore` in the environment.
public struct LocalAppsSheet: View {
    @Environment(BotStore.self) private var store
    @State private var apps: LocalApps?
    @State private var busy = false
    @State private var error: String?

    public init() {}

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Local apps") {
                if busy || (apps == nil && error == nil) { Spinner() }
            }
            CardForm {
                if let apps {
                    CardSection(footer: "A local safeguard on this computer, not a plan: the bots here may keep up to this many mini-apps between them. Changing an app a bot already made never counts, and deleting one makes room.") {
                        ValueRow("Apps on this computer") { Text(countText(apps)) }
                        ValueRow("Bots that made apps") { Text(String(apps.botsWithApps)) }
                        ValueRow("Local app limit") {
                            ChoicePicker(
                                selection: Binding(get: { apps.limit }, set: { save($0) }),
                                options: apps.choices.map { (id: $0, label: String($0)) }
                            )
                        }
                    }
                } else if let error {
                    // No limit to show: what the computer said stands in for the card.
                    CardSection { Text(L10n.tr(error)).foregroundStyle(Palette.danger).textSelection(.enabled) }
                } else {
                    CardSection { Text(L10n.tr("Loading…")).foregroundStyle(Palette.secondary) }
                }
                // A limit the computer refused: the card keeps what it holds and the reason shows under it.
                if apps != nil, let error {
                    Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                }
            }
        }
        .background(Palette.background)
        .presentationDetents([.medium])
        .task { await load() }
    }

    /// "7 of 100", laid out by the language.
    private func countText(_ apps: LocalApps) -> String {
        L10n.tr("{used} of {limit}")
            .replacingOccurrences(of: "{used}", with: String(apps.count))
            .replacingOccurrences(of: "{limit}", with: String(apps.limit))
    }

    private func load() async {
        do {
            apps = try await store.localApps()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func save(_ limit: Int) {
        guard !busy, limit != apps?.limit else { return }
        busy = true
        error = nil
        Task {
            do {
                try await store.setLocalAppsLimit(limit)
                apps = try await store.localApps()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
