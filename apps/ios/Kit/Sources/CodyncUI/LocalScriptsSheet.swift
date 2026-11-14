import CodyncKit
import SwiftUI

/// The hourly ceiling on requests local scripts send to routines of this computer, shared by every agent.
/// Past it the local address answers 429 and the script tries again later.
struct LocalScriptsSheet: View {
    @Environment(BotStore.self) private var store
    @State private var limit: ScriptLimit?
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Local scripts")
            CardForm {
                if let limit {
                    CardSection(footer: "How many requests a script on this computer may send to routines each hour, for every agent together. Past it the address answers 429 and the script tries again later.") {
                        ValueRow("Requests an hour") {
                            ChoicePicker(
                                selection: Binding(get: { limit.perHour }, set: { save($0) }),
                                options: limit.choices.map { (id: $0, label: String($0)) }
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
                limit = try await store.scriptLimit()
            } catch {
                self.error = error.localizedDescription
            }
        }
    }

    private func save(_ perHour: Int) {
        guard !busy, perHour != limit?.perHour else { return }
        busy = true
        error = nil
        Task {
            do {
                try await store.setScriptLimit(perHour)
                limit = try await store.scriptLimit()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
