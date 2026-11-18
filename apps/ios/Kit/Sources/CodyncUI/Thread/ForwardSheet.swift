import CodyncKit
import SwiftUI

/// Picks the chat a message is sent on to ("Send to a channel"). The host copies the message and
/// its files into the target, marked as forwarded, so it stays there even if this chat is deleted.
struct ForwardSheet: View {
    let entry: Entry
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var query = ""
    @State private var busy: String?
    @State private var error: String?

    /// Every chat on this computer by name: a bot's own chat or a group, which is what a message
    /// goes into. The host refuses a target the message can't be copied to.
    private var chats: [Bot] {
        model.roster
            .filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) }
            .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Send to a channel")
            Text(L10n.tr("The message arrives there as a forwarded copy."))
                .font(.caption)
                .foregroundStyle(Palette.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 20)
                .padding(.bottom, 12)
            if !model.roster.isEmpty {
                SearchField("Search", text: $query)
                    .padding(.horizontal, 20)
                    .padding(.bottom, 12)
            }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 14) {
                    if chats.isEmpty {
                        Text(model.roster.isEmpty ? L10n.tr("No chats yet.") : L10n.tr("No matches."))
                            .font(.footnote)
                            .foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    } else {
                        CardSection {
                            ForEach(chats) { bot in
                                Button { send(bot) } label: { row(bot) }
                                    .buttonStyle(PressScale())
                                    .disabled(busy != nil)
                            }
                        }
                    }
                    if let error {
                        Text(error)
                            .font(.footnote)
                            .foregroundStyle(Palette.danger)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                .padding(20)
            }
            .scrollDismissesKeyboard(.interactively)
        }
        .background(Palette.background)
    }

    private func row(_ bot: Bot) -> some View {
        HStack(spacing: 12) {
            if bot.isGroup {
                GroupAvatar(members: model.members(of: bot), size: 32, animated: false)
            } else {
                CharacterAvatar(bot: bot, size: 32, animated: false)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(bot.name).foregroundStyle(Palette.text).lineLimit(1)
                if bot.isGroup {
                    Text(L10n.tr("Group chat")).font(.caption).foregroundStyle(Palette.secondary)
                }
            }
            Spacer(minLength: 8)
            if busy == bot.id { Spinner(size: 14) }
        }
        .frame(minHeight: 40)
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
