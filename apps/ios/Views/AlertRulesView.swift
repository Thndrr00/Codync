import CodyncUI
import SwiftUI

/// The computer's alert rules: which kinds of turn reach this phone, and the quiet hours that hold
/// everything back except an approval. iPhone reach for host/src/store/notification_rules.rs.
struct AlertRulesSheet: View {
    @Environment(BotStore.self) private var store
    @State private var rules: AlertRules?
    @State private var error: String?

    private static let hours: [(id: Int, label: String)] = (0..<24).map { hour in
        let shown = hour == 0 ? 12 : hour > 12 ? hour - 12 : hour
        return (id: hour, label: "\(shown) \(hour < 12 ? "AM" : "PM")")
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Alert rules") {
                if rules == nil { Spinner() }
            }
            CardForm {
                CardSection("Alerts", footer: "An approval always arrives, even in quiet hours.") {
                    if let current = rules {
                        rule("Task finished", current.done) { value in change { $0.done = value } }
                        rule("Needs you", current.needsInput) { value in change { $0.needsInput = value } }
                        rule("Task failed", current.failed) { value in change { $0.failed = value } }
                        rule("Task paused", current.paused) { value in change { $0.paused = value } }
                    } else if let error {
                        Text(error).font(.footnote).foregroundStyle(Palette.danger)
                    } else {
                        Text("Loading…").foregroundStyle(Palette.secondary)
                    }
                }
                if let current = rules {
                    CardSection("Quiet hours", footer: "Finished, failed and paused tasks wait until quiet hours end.") {
                        Toggle(isOn: Binding(get: { current.hasQuietHours },
                                             set: { on in change { rules in
                                                 rules.quietStart = on ? 22 : 0
                                                 rules.quietEnd = on ? 8 : 0
                                             } })) {
                            Text("Quiet hours").foregroundStyle(Palette.text)
                        }
                        .toggleStyle(.codync)
                        if current.hasQuietHours {
                            HStack {
                                Text("From").foregroundStyle(Palette.text)
                                Spacer()
                                ChoicePicker(selection: Binding(get: { current.quietStart },
                                                                set: { hour in change { $0.quietStart = hour } }),
                                             options: Self.hours)
                            }
                            HStack {
                                Text("Until").foregroundStyle(Palette.text)
                                Spacer()
                                ChoicePicker(selection: Binding(get: { current.quietEnd },
                                                                set: { hour in change { $0.quietEnd = hour } }),
                                             options: Self.hours)
                            }
                        }
                    }
                }
                if let error, rules != nil {
                    Text(error).font(.footnote).foregroundStyle(Palette.danger)
                }
            }
        }
        .task { await load() }
    }

    private func rule(_ title: String, _ on: Bool, set: @escaping (Bool) -> Void) -> some View {
        Toggle(isOn: Binding(get: { on }, set: set)) {
            Text(title).foregroundStyle(Palette.text)
        }
        .toggleStyle(.codync)
    }

    private func change(_ edit: (inout AlertRules) -> Void) {
        guard var next = rules else { return }
        edit(&next)
        rules = next
        Task {
            do {
                rules = try await store.setAlertRules(next)
                error = nil
            } catch {
                self.error = errorText(error)
            }
        }
    }

    private func load() async {
        do { rules = try await store.alertRules() } catch { error = errorText(error) }
    }

    private func errorText(_ error: Error) -> String {
        (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
}
