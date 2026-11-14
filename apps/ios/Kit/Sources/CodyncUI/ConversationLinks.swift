import CodyncKit
import SwiftUI

/// The Codync thread a channel conversation shares (card K13): what is linked now, a way to link one, and to take
/// it away. The computer keys a conversation by its own ids (a Telegram chat id, a Slack thread timestamp, a Discord
/// channel), so only a Discord channel is the one the page shows; the other two are typed in.
struct ConversationLinksView: View {
    let provider: String
    /// The channel this page shows: a Discord channel's id.
    let channel: String
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var links: [ConversationLink] = []
    @State private var threads: [ThreadInfo] = []
    @State private var conversation = ""
    @State private var choice = ""
    @State private var busy = false
    @State private var error: String?

    /// The host's name for this conversation: a Telegram chat or a Slack thread is its id, a Discord channel is here.
    private var key: (channel: String, thread: String) {
        let typed = conversation.trimmingCharacters(in: .whitespacesAndNewlines)
        if provider == "discord" { return (channel, "") }
        if provider == "slack" { return (channel, typed) }
        return (typed, "")
    }

    /// A page shows its own channel's links; a Telegram bot's conversations are all its chats, which it names by id.
    private var mine: [ConversationLink] {
        links.filter { $0.source == provider && (provider == "telegram" || $0.channel == channel) }
    }

    private func name(of rootId: String) -> String {
        threads.first { $0.id == rootId }?.name ?? rootId
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(L10n.tr("Linked conversation")).font(.caption).foregroundStyle(Palette.tertiary)
            ForEach(mine) { link in
                HStack(spacing: 8) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(L10n.tr("Linked to {thread}").replacingOccurrences(of: "{thread}", with: name(of: link.rootId)))
                            .font(.caption)
                        Text(link.thread.isEmpty ? link.channel : "\(link.channel) · \(link.thread)")
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    IconButton("Unlink", systemImage: "minus.circle") {
                        act { try await model.unlinkConversation(botId: botId, provider: provider, channel: link.channel, thread: link.thread) }
                    }
                    .disabled(busy)
                }
            }
            if provider != "discord" {
                TextField(L10n.tr("Its id in the channel: a Telegram chat id, a Slack thread timestamp"), text: $conversation)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .fieldBox()
            }
            HStack(spacing: 8) {
                ChoicePicker(selection: $choice, options: threads.map { (id: $0.id, label: $0.name) })
                Button("Link a thread") {
                    let target = key
                    act { try await model.linkConversation(botId: botId, provider: provider, channel: target.channel, thread: target.thread, threadId: choice) }
                }
                .buttonStyle(.secondary)
                .disabled(busy || choice.isEmpty || key.channel.isEmpty)
            }
            if let error {
                Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger)
            }
        }
        .task(id: "\(provider)/\(channel)/\(botId)") { await load() }
    }

    private func load() async {
        links = (try? await model.conversationLinks(botId)) ?? []
        threads = (try? await model.channelThreads(botId)) ?? []
    }

    /// One change at a time; the computer's answer replaces what is shown.
    private func act(_ work: @escaping @MainActor () async throws -> [ConversationLink]) {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                links = try await work()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
