import CodyncKit
import SwiftUI

/// The mini-apps a bot built, read from the host through the existing client bridge. The host owns
/// the apps and their capability links; this view only renders and asks for them.
struct MiniAppsSection: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var apps: [BotApp] = []
    @State private var error: String?
    @State private var sharedURL: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("Mini-apps"))
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(Palette.text)
                .padding(.leading, 2)
            if let sharedURL {
                Text(sharedURL).font(.footnote).foregroundStyle(Palette.secondary).textSelection(.enabled)
            }
            if let error {
                Text(error).font(.footnote).foregroundStyle(Palette.secondary)
            } else if apps.isEmpty {
                Text(L10n.tr("Ask the bot to build an app and it will appear here."))
                    .font(.footnote).foregroundStyle(Palette.secondary)
            } else {
                ForEach(apps) { app in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(app.title).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                        Text(L10n.tr("Version {version}").replacingOccurrences(of: "{version}", with: String(app.version)))
                            .font(.footnote).foregroundStyle(Palette.secondary)
                        HStack(spacing: 12) {
                            Button(L10n.tr("Share view link")) { share(app, mode: .view) }
                            Button(L10n.tr("Share edit link")) { share(app, mode: .edit) }
                        }
                        .font(.footnote)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .outlinedCard()
                }
            }
        }
        .task { await load() }
    }

    private func load() async {
        guard let client = model.client else { return }
        do {
            apps = try await client.listBotApps(botId: botId)
            error = nil
        } catch {
            self.error = L10n.tr("Couldn't load the bot's apps.")
        }
    }

    private func share(_ app: BotApp, mode: AppLinkMode) {
        Task {
            guard let client = model.client else { return }
            do {
                let created = try await client.createAppLink(
                    botId: botId, slug: app.slug, label: "iPhone \(mode.rawValue)", mode: mode
                )
                sharedURL = created.url
                await load()
            } catch {
                self.error = L10n.tr("Couldn't create the link.")
            }
        }
    }
}
