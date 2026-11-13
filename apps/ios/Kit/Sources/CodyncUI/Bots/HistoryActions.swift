import CodyncKit
import SwiftUI

/// A chat's or group's history actions, in its settings: Delete history removes its messages and turns
/// only; New conversation keeps them and starts the agents fresh; Compact context asks the agent (or
/// every member of a group) to summarize what it keeps. Ported from OpenBot.
struct HistoryActions: View {
    let bot: Bot
    @Environment(BotStore.self) private var model
    @State private var confirming = false
    @State private var typed = ""

    var body: some View {
        CardSection(
            "History",
            footer: "Deleting the history removes this chat's messages and turns. Its setup, memories, routines, artifacts and files stay."
        ) {
            row("Delete history", icon: "trash", destructive: true) {
                typed = ""
                confirming = true
            }
            row("New conversation", icon: "arrow.counterclockwise") {
                model.newSession(bot.id)
            }
            row("Compact context", icon: "rectangle.compress.vertical") {
                model.compactContext(bot)
            }
        }
        .codyncDialog(
            L10n.tr("Delete {name}'s history?").replacingOccurrences(of: "{name}", with: bot.name),
            isPresented: $confirming,
            message: L10n.tr("Its messages and turns go. Its setup, memories, routines, artifacts and files stay."),
            field: DialogField(L10n.tr("Type {name} to confirm").replacingOccurrences(of: "{name}", with: bot.name),
                               text: $typed)
        ) {
            [
                DialogAction("Delete history", destructive: true,
                             enabled: typed.trimmingCharacters(in: .whitespacesAndNewlines) == bot.name) {
                    model.deleteHistory(bot)
                },
            ]
        }
    }

    private func row(_ title: String, icon: String, destructive: Bool = false,
                     action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .font(.system(size: 15))
                    .foregroundStyle(destructive ? Palette.danger : Palette.secondary)
                    .frame(width: 22)
                Text(L10n.tr(title))
                    .foregroundStyle(destructive ? Palette.danger : Palette.text)
                Spacer(minLength: 0)
            }
            .frame(minHeight: 32)
            .contentShape(Rectangle())
        }
        .buttonStyle(PressScale())
    }
}
