import CodyncKit
import SwiftUI

/// The computer-wide ceiling on local mini-apps: a safeguard on this computer, not a plan. Only a new
/// app takes a slot, and a bot may always update an app it already made.
public struct LocalAppsSheet: View {
    @Environment(BotStore.self) private var store
    @State private var apps: LocalApps?
    @State private var busy = false
    @State private var error: String?

    public init() {}

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Local apps")
            CardForm {
                if let apps {
                    CardSection(footer: "A local safeguard on this computer, not a plan: the bots here may keep up to this many mini-apps between them. Changing an app a bot already made never counts, and deleting one makes room.") {
                        ValueRow("Apps on this computer") {
                            Text(L10n.fill(L10n.tr("{used} of {limit}"), ["used": String(apps.count), "limit": String(apps.limit)]))
                        }
                        ValueRow("Local app limit") {
                            ChoicePicker(
                                selection: Binding(get: { apps.limit }, set: { save($0) }),
                                options: apps.choices.map { (id: $0, label: String($0)) }
                            )
                        }
                    }
                } else {
                    CardSection { Text(L10n.tr("Loading…")).foregroundStyle(Palette.secondary) }
                }
                if let error {
                    Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                }
                if busy {
                    Text(L10n.tr("Saving…")).font(.caption).foregroundStyle(Palette.secondary)
                }
            }
        }
        .background(Palette.background)
        .presentationDetents([.medium])
        .task {
            do {
                apps = try await store.localApps()
            } catch {
                self.error = error.localizedDescription
            }
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
