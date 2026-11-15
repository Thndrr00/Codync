import CodyncKit
import CodyncUI
import SwiftUI

/// Picks the bot a shared link or text goes to. It only fills that bot's message box; sending stays
/// with the person. iPhone counterpart of Android's share target.
struct ShareBotPicker: View {
    let intake: SharedIntake
    @Environment(AppStore.self) private var app
    @Environment(AccountStore.self) private var accounts
    @Environment(\.dismissModal) private var dismiss

    private var bots: [(store: BotStore, bot: Bot)] {
        accounts.computers.flatMap { computer -> [(BotStore, Bot)] in
            guard let store = accounts.store(for: computer.id), store.connection == .online else { return [] }
            return store.roster.filter { !$0.deleted && $0.kind != "group" }.map { (store, $0) }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Send to a bot")
            CardForm {
                CardSection(footer: "It becomes a draft; nothing is sent until you tap Send.") {
                    if !intake.text.isEmpty {
                        Text(intake.text).font(.callout).foregroundStyle(Palette.secondary).lineLimit(4)
                    }
                    if !intake.files.isEmpty {
                        Text(intake.files.map(\.name).joined(separator: ", "))
                            .font(.footnote)
                            .foregroundStyle(Palette.secondary)
                    }
                }
                if bots.isEmpty {
                    CardSection {
                        Text("No computer is online. Open Codync when one is.").foregroundStyle(Palette.secondary)
                    }
                } else {
                    CardSection("Bots") {
                        ForEach(bots, id: \.bot.id) { pair in
                            Button {
                                pair.store.appendShared(intake.text, to: pair.bot.id)
                                pair.store.addSharedFiles(intake.files.map { OutgoingFile(name: $0.name, data: $0.data) },
                                                          to: pair.bot.id)
                                app.shared(intake, to: pair.bot, on: pair.store)
                                dismiss()
                            } label: {
                                HStack(spacing: 10) {
                                    CharacterAvatar(bot: pair.bot, size: 30)
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(pair.bot.name).foregroundStyle(Palette.text)
                                        Text(pair.store.computer.name).font(.caption).foregroundStyle(Palette.secondary)
                                    }
                                    Spacer(minLength: 0)
                                }
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
        }
    }
}
