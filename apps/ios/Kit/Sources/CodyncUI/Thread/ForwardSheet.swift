import CodyncKit
import SwiftUI

/// Picks the chat a message is sent on to ("Send to a channel"). The host copies the message and its files into
/// the target, marked as forwarded, so it stays there even if this chat is deleted.
struct ForwardSheet: View {
    let entry: Entry
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var busy: String?
    @State private var error: String?

    private var bots: [Bot] {
        model.roster.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Send to a channel")
            Text(L10n.tr("The message arrives there as a forwarded copy."))
                .font(.caption)
                .foregroundStyle(Palette.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.vertical, 4)
            Hairline()
            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(bots) { bot in
                        Button { send(bot) } label: { row(bot) }
                            .buttonStyle(PressScale())
                            .disabled(busy != nil)
                    }
                    if bots.isEmpty {
                        Text(L10n.tr("No chats yet."))
                            .foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, 12)
                    }
                    if let error {
                        Text(error)
                            .font(.footnote)
                            .foregroundStyle(Palette.danger)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.top, 8)
                    }
                }
                .padding(16)
            }
        }
        .background(Palette.background)
    }

    private func row(_ bot: Bot) -> some View {
        HStack(spacing: 14) {
            if bot.isGroup {
                GroupAvatar(members: model.members(of: bot), size: 32, photo: bot.avatarPhoto)
            } else {
                CharacterAvatar(bot: bot, size: 32, animated: false)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(bot.name).foregroundStyle(Palette.text).lineLimit(1)
                if bot.isGroup {
                    Text(L10n.tr("Group chat")).font(.caption).foregroundStyle(Palette.secondary)
                }
            }
            Spacer(minLength: 0)
            if busy == bot.id { Spinner(size: 14) }
        }
        .frame(minHeight: 48)
        .contentShape(Rectangle())
    }

    private func send(_ bot: Bot) {
        guard busy == nil else { return }
        busy = bot.id
        error = nil
        Task {
            do {
                try await model.forward(entry, to: bot.id)
                dismiss()
            } catch {
                self.error = error.localizedDescription
                busy = nil
            }
        }
    }
}
