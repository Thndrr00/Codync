import CodyncKit
import SwiftUI

/// When a plan card of a terminal agent starts the plan by itself (`planStart`), and what it may be.
struct PlanStartSetting: Decodable, Sendable {
    /// `nil` waits until the person presses Start.
    let afterMs: Int?
    let presets: [Int]
    let min: Int
    let max: Int
}

extension BotStore {
    func planStart() async throws -> PlanStartSetting {
        try await ready().call("planStart")
    }

    /// Saves when a plan starts by itself; `nil` waits for the person. Throws the computer's reason when too short.
    func setPlanStart(afterMs: Int?) async throws {
        struct Body: Encodable, Sendable {
            let afterMs: Int?
        }
        struct Saved: Decodable, Sendable {
            let afterMs: Int?
        }
        let _: Saved = try await ready().call("setPlanStart", Body(afterMs: afterMs))
    }
}

/// " · 4:59" on a plan card's Start button when the plan starts by itself in a minute or more.
func planCountdown(_ at: Int64, now: Date) -> String {
    let remaining = at - Int64(now.timeIntervalSince1970 * 1_000)
    let total = max(0, Int((Double(remaining) / 1_000).rounded(.up)))
    guard total >= 60 else { return "" }
    return String(format: " · %d:%02d", total / 60, total % 60)
}

/// "10 s", "1 min", "1 min 30 s".
func planDelayText(_ ms: Int) -> String {
    let minutes = ms / 60_000
    let seconds = Int((Double(ms % 60_000) / 1_000).rounded())
    if minutes == 0 { return "\(seconds) s" }
    if seconds == 0 { return "\(minutes) min" }
    return "\(minutes) min \(seconds) s"
}

/// "Start the work": when a plan card starts the plan by itself, for every agent on this computer.
struct PlanStartSheet: View {
    @Environment(BotStore.self) private var store
    @State private var setting: PlanStartSetting?
    @State private var choice = "wait"
    @State private var busy = false
    @State private var error: String?
    @State private var customOpen = false
    @State private var amount = "1"
    @State private var unit = "min"

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Plan cards")
            CardForm {
                if let setting {
                    CardSection(footer: footer(setting)) {
                        ValueRow("Start the work") {
                            ChoicePicker(selection: $choice, options: options(setting), fitsAvailableWidth: true)
                                .onChange(of: choice) { _, id in pick(id, setting) }
                        }
                        if customOpen {
                            ValueRow("Time before the work starts") {
                                HStack(spacing: 8) {
                                    TextField("5", text: $amount)
                                        .keyboardType(.numberPad)
                                        .multilineTextAlignment(.trailing)
                                        .frame(maxWidth: 70)
                                    ChoicePicker(
                                        selection: $unit,
                                        options: [(id: "s", label: L10n.tr("sec")), (id: "min", label: L10n.tr("min"))],
                                        fitsAvailableWidth: true
                                    )
                                }
                            }
                            HStack(spacing: 8) {
                                Text(custom(setting) == nil
                                     ? L10n.tr("Enter a whole number of at least 5 seconds.")
                                     : L10n.tr("At least 5 seconds."))
                                    .font(.footnote)
                                    .foregroundStyle(custom(setting) == nil ? Palette.danger : Palette.secondary)
                                Spacer(minLength: 0)
                                Button(L10n.tr("Set")) {
                                    if let ms = custom(setting) { save(ms) }
                                }
                                .buttonStyle(.plain)
                                .foregroundStyle(Palette.accent)
                                .disabled(custom(setting) == nil || busy)
                                Button(L10n.tr("Cancel")) { customOpen = false }
                                    .buttonStyle(.plain)
                                    .foregroundStyle(Palette.secondary)
                            }
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
        .presentationDetents([.medium, .large])
        .task {
            do {
                setting = try await store.planStart()
                choice = choiceID(setting)
            } catch {
                self.error = error.localizedDescription
            }
        }
    }

    private func choiceID(_ setting: PlanStartSetting) -> String {
        setting.afterMs.map(String.init) ?? "wait"
    }

    private func pick(_ id: String, _ setting: PlanStartSetting) {
        guard id != choiceID(setting) else { return }
        if id == "custom" {
            customOpen = true
        } else {
            save(id == "wait" ? nil : Int(id))
        }
    }

    private func options(_ setting: PlanStartSetting) -> [(id: String, label: String)] {
        var list: [(id: String, label: String)] = [(id: "wait", label: L10n.tr("When I press Start"))]
        for ms in setting.presets {
            list.append((id: String(ms), label: L10n.fill(L10n.tr("After {time}"), ["time": planDelayText(ms)])))
        }
        if let after = setting.afterMs, !setting.presets.contains(after) {
            list.append((id: String(after), label: L10n.fill(L10n.tr("After {time}"), ["time": planDelayText(after)])))
        }
        list.append((id: "custom", label: L10n.tr("Custom time…")))
        return list
    }

    private func footer(_ setting: PlanStartSetting) -> String {
        guard let after = setting.afterMs else {
            return L10n.tr("The plan waits until you press Start. You get a notification.")
        }
        return L10n.fill(L10n.tr("The work starts after {time}, or when you press Start."), ["time": planDelayText(after)])
    }

    /// The typed time, or `nil` when it isn't a whole number in the allowed range.
    private func custom(_ setting: PlanStartSetting) -> Int? {
        let trimmed = amount.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty, trimmed.allSatisfy(\.isNumber), let value = Int(trimmed) else { return nil }
        let ms = value * (unit == "min" ? 60_000 : 1_000)
        return ms >= setting.min && ms <= setting.max ? ms : nil
    }

    private func save(_ afterMs: Int?) {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                try await store.setPlanStart(afterMs: afterMs)
                setting = try await store.planStart()
                if let setting { choice = choiceID(setting) }
                customOpen = false
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
