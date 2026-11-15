import CodyncKit
import SwiftUI

/// The host's read-only answer to "what would happen?" for a proposed tool call (`previewApproval`):
/// the same approval rules a real request goes through, and the rule or default that decided.
struct ApprovalPreview: Decodable, Sendable {
    let decision: String
    let source: String
    let explanation: String
}

extension BotStore {
    func previewApproval(botId: String, tool: String, command: String) async throws -> ApprovalPreview {
        struct Body: Encodable { var botId: String; var tool: String; var command: String? }
        struct Reply: Decodable, Sendable { var preview: ApprovalPreview }
        let reply: Reply = try await ready().call(
            "previewApproval",
            Body(botId: botId, tool: tool, command: command.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? nil : command)
        )
        return reply.preview
    }
}

/// A small panel in a bot's approvals settings. It only asks the host; nothing is sent to the bot.
struct ApprovalWhatIf: View {
    let botId: String
    @Environment(BotStore.self) private var store
    @State private var tool = "shell"
    @State private var command = ""
    @State private var answer: String?
    @State private var busy = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(L10n.tr("What would happen?")).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
            Text(L10n.tr("Name a tool and what it would be asked to do. The host answers with the same rules; nothing runs."))
                .font(.caption)
                .foregroundStyle(Palette.secondary)
            TextField(L10n.tr("Tool"), text: $tool)
                .textFieldStyle(.plain)
                .textInputAutocapitalization(.never)
            TextField(L10n.tr("Command or input"), text: $command, axis: .vertical)
                .textFieldStyle(.plain)
                .lineLimit(1...3)
                .textInputAutocapitalization(.never)
            Button(busy ? L10n.tr("Asking…") : L10n.tr("Ask")) { ask() }
                .buttonStyle(.secondary)
                .disabled(busy || tool.trimmingCharacters(in: .whitespaces).isEmpty)
            if let answer {
                Text(answer).font(.caption).foregroundStyle(Palette.secondary).textSelection(.enabled)
            }
        }
        .padding(.vertical, 4)
    }

    private func ask() {
        guard !busy else { return }
        busy = true
        answer = nil
        Task {
            do {
                let preview = try await store.previewApproval(botId: botId, tool: tool, command: command)
                answer = "\(preview.decision.capitalized). \(preview.explanation)"
            } catch {
                answer = error.localizedDescription
            }
            busy = false
        }
    }
}
