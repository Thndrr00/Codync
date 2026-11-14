import CodyncKit
import SwiftUI

/// The chat's pinned messages, newest first, across the chat and its threads. A pin opens its thread;
/// the agent's pins are taken back here like its own tool takes them back.
struct PinsSheet: View {
    let botId: String
    /// Tells the chat to open a thread.
    let open: (String) -> Void
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss

    private var pins: [PinnedMessage] { model.pins[botId] ?? [] }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Pinned messages")
            Text(L10n.tr("Kept at hand in this chat. A message can be unpinned by whoever reads it here."))
                .font(.caption)
                .foregroundStyle(Palette.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.vertical, 4)
            Hairline()
            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(pins) { pin in
                        PinRow(pin: pin, agent: model.bots[botId]?.name ?? L10n.tr("The agent"),
                               author: authorName(pin), open: { show(pin) }, unpin: { model.unpin(pin, botId: botId) })
                    }
                    if pins.isEmpty {
                        Text(L10n.tr("Nothing is pinned here yet. Open a message's menu and pick Pin."))
                            .font(.body)
                            .foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, 12)
                    }
                }
                .padding(16)
            }
        }
        .background(Palette.background)
        .task { model.refreshPins(botId) }
    }

    private func authorName(_ pin: PinnedMessage) -> String {
        pin.author == "user" ? L10n.tr("You") : model.bots[pin.authorBotId ?? botId]?.name ?? L10n.tr("The agent")
    }

    /// Closes this list, then opens the thread once it is gone: two sheets don't replace each other.
    private func show(_ pin: PinnedMessage) {
        guard let root = pin.threadId else { return }
        dismiss()
        Task {
            try? await Task.sleep(for: .milliseconds(250))
            open(root)
        }
    }
}

private struct PinRow: View {
    let pin: PinnedMessage
    let agent: String
    let author: String
    let open: () -> Void
    let unpin: () -> Void

    private var who: String {
        if pin.pinnedBy.kind == "agent" { return L10n.tr("{name} (agent)").replacingOccurrences(of: "{name}", with: agent) }
        return pin.pinnedBy.device ?? L10n.tr("You")
    }

    var body: some View {
        HStack(spacing: 10) {
            Button(action: open) {
                VStack(alignment: .leading, spacing: 2) {
                    if let title = pin.threadTitle {
                        Text(title).font(.caption).foregroundStyle(Palette.tertiary).lineLimit(1)
                    }
                    Text(pin.text.isEmpty ? L10n.tr("A message without text") : pin.text)
                        .font(.body)
                        .foregroundStyle(Palette.text)
                        .lineLimit(3)
                    Text("\(who) · \(author)").font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            IconButton("Unpin", systemImage: "pin.slash", action: unpin)
        }
        .padding(.vertical, 8)
    }
}
