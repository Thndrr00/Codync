import CodyncKit
import SwiftUI

public struct RosterPreviewView: View {
    let bot: Bot
    let store: BotStore
    @State private var preview = RosterPreview()

    public init(bot: Bot, store: BotStore) {
        self.bot = bot
        self.store = store
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(bot.name).font(.headline)
            Text("Main chat preview · unread unchanged").font(.caption).foregroundStyle(Palette.secondary)
            if let error = preview.error { Text(error).foregroundStyle(Palette.warning) }
            else if let entries = preview.entries {
                if entries.isEmpty { Text("No messages yet").foregroundStyle(Palette.secondary) }
                ForEach(entries) { Text($0.data.text ?? "").font(.subheadline).lineLimit(3) }
            } else { Text("Loading preview…").foregroundStyle(Palette.secondary) }
        }
        .padding(16).frame(maxWidth: 360, alignment: .leading).background(Palette.surface)
        .task(id: bot.id) { await preview.load(bot.id, store: store) }
    }
}
