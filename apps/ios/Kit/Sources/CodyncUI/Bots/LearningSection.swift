import CodyncKit
import SwiftUI

/// A durable preference a bot proposes from the person's own messages (`learning`). Nothing is memory until approved.
struct LearningProposal: Codable, Identifiable, Hashable {
    let id: String
    let botId: String?
    let text: String
    /// `correction`, `instruction` or `repetition`.
    let kind: String
    /// How many times the person said it.
    let hits: Int
    /// The message it came from, cut short.
    let source: String

    var kindLabel: String {
        switch kind {
        case "correction": "Correction"
        case "repetition": hits > 1 ? "Said again · \(hits) times" : "Said again"
        default: "Preference"
        }
    }
}

struct LearningInbox: Codable {
    var enabled: Bool
    var autoApproveAfter: Int
    var maxAutoApproveAfter: Int
    var note: String
    var proposals: [LearningProposal]
}

private struct LearningRequest: Encodable {
    let botId: String
    var id: String? = nil
    var enabled: Bool? = nil
    var autoApproveAfter: Int? = nil
}

/// What a bot learned from the person's corrections and repeated instructions: one card each, saved only on approval.
struct LearningSection: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var inbox: LearningInbox?
    @State private var error: String?
    @State private var busy = false

    private static let repeats = [0, 2, 3, 5]

    var body: some View {
        CardSection(title, footer: footer) {
            if let inbox {
                SwitchRow("Learn from my corrections and instructions",
                          detail: "Proposes durable preferences, never one-off tasks or secrets.",
                          isOn: Binding(get: { inbox.enabled }, set: { save(enabled: $0) }))
                Text(L10n.tr("Save without review after"))
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
                TabChoice(selection: Binding(get: { inbox.autoApproveAfter }, set: { save(auto: $0) }),
                          options: Self.repeats.map { ($0, $0 == 0 ? "Off" : "\($0)×") })
                if inbox.proposals.isEmpty {
                    Text(L10n.tr("Nothing to review.")).foregroundStyle(Palette.secondary)
                }
                ForEach(inbox.proposals) { proposal in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(proposal.text).foregroundStyle(Palette.text)
                        Text(proposal.kindLabel).font(.caption).foregroundStyle(Palette.tertiary)
                        if !proposal.source.isEmpty {
                            Text("\(L10n.tr("From")): “\(proposal.source)”")
                                .font(.footnote)
                                .foregroundStyle(Palette.secondary)
                                .lineLimit(2)
                        }
                        HStack(spacing: 8) {
                            Button(L10n.tr("Remember this")) { act("approveLearning", id: proposal.id) }
                                .buttonStyle(.primary)
                                .disabled(busy)
                            Button(L10n.tr("Dismiss")) { act("dismissLearning", id: proposal.id) }
                                .buttonStyle(.secondary)
                                .disabled(busy)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            if let error {
                Text(error).font(.footnote).foregroundStyle(Palette.danger)
            }
        }
        .task(id: botId) { await load("learning") }
    }

    private var title: String {
        let count = inbox?.proposals.count ?? 0
        return count > 0 ? "Learned preferences (\(count))" : "Learned preferences"
    }

    private var footer: String {
        "Corrections and instructions you give this bot can become one line of its \(inbox?.note ?? "preferences.md") note. Nothing is saved until you approve it, unless you let it save after repeats. The bot's replies, routines and group chats are never read for this."
    }

    private func load(_ method: String, _ ask: LearningRequest? = nil) async {
        guard let client = model.client, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let reply: LearningInbox = try await client.call(method, ask ?? LearningRequest(botId: botId))
            inbox = reply
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func save(enabled: Bool? = nil, auto: Int? = nil) {
        guard !busy else { return }
        Task {
            await load("saveLearningSettings", LearningRequest(botId: botId, enabled: enabled, autoApproveAfter: auto))
        }
    }

    private func act(_ method: String, id: String) {
        guard !busy else { return }
        Task { await load(method, LearningRequest(botId: botId, id: id)) }
    }
}
