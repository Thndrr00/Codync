import CodyncKit
import SwiftUI

/// API keys and command variables bots asked for on a secure card; values stay hidden.
struct BotSecretsSection: View {
    @Environment(BotStore.self) private var model
    @State private var secrets: [BotSecret] = []
    @State private var error: String?

    var body: some View {
        CardSection("Bot secrets", footer: "Bots ask for these on a secure card. Codync attaches them to the bot's API calls or commands; the bot never sees the value.") {
            ForEach(secrets) { secret in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(secret.name)
                        Text("\(secret.botName ?? "A removed bot") · \(secret.destination)")
                            .font(.caption).foregroundStyle(Palette.secondary)
                    }
                    Spacer()
                    IconButton("Remove", systemImage: "trash") { remove(secret) }
                }
            }
            if secrets.isEmpty {
                Text("No bot secrets yet.").font(.caption).foregroundStyle(Palette.secondary)
            }
            if let error { Text(error).font(.footnote).foregroundStyle(Palette.danger) }
        }
        .task { await load() }
    }

    private func load() async {
        do { secrets = try await model.client?.botSecrets() ?? [] } catch { self.error = error.localizedDescription }
    }

    private func remove(_ secret: BotSecret) {
        guard let client = model.client else { return }
        Task {
            do {
                try await client.removeBotSecret(secret.id)
                withAnimation(Motion.layout) { secrets.removeAll { $0.id == secret.id } }
            } catch { self.error = error.localizedDescription }
        }
    }
}
