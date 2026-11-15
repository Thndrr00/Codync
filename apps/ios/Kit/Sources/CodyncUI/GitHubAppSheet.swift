import CodyncKit
import SwiftUI

/// The computer's GitHub App, read-only: whether it is connected, where it is installed and which
/// repositories reach which bots. It is set up at the computer, where its key stays.
/// The computer is the `BotStore` in the environment.
public struct GitHubAppSheet: View {
    @Environment(BotStore.self) private var store

    public init() {}

    private let footer = "An App of your own signs its deliveries, so a routine runs on pull requests, reviews, checks and issues without a person's token. Required permissions: contents and pull requests read, checks read. Events: pull request, pull request review, check run, issues. The key and the webhook secret stay in this computer's credential vault."

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader { Text(L10n.tr("GitHub App")) }
            CardForm {
                if let app = store.githubApp, app.configured {
                    configured(app)
                } else if store.githubApp != nil {
                    CardSection(footer: footer) {
                        Text(L10n.tr("No GitHub App yet. Connect it in the desktop app."))
                            .font(.subheadline).foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
                    }
                } else {
                    CardSection(footer: footer) {
                        Text(L10n.tr("Loading…"))
                            .font(.subheadline).foregroundStyle(Palette.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
                    }
                }
            }
        }
        .background(Palette.background)
        .task { await store.refreshGitHubApp() }
    }

    @ViewBuilder
    private func configured(_ app: GitHubAppStatus) -> some View {
        CardSection(footer: footer) {
            ValueRow("App", value: app.appId ?? "")
            Text("\(app.installations.count) \(L10n.tr("installations")) · \(app.repos.count) \(L10n.tr("repositories mapped"))")
                .font(.caption).foregroundStyle(Palette.tertiary)
                .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
            if app.installations.isEmpty {
                Text(L10n.tr("Not installed anywhere yet."))
                    .font(.caption).foregroundStyle(Palette.tertiary)
                    .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
            }
        }
        ForEach(app.installations) { installation in
            CardSection(installation.account) {
                ForEach(installation.repos, id: \.self) { repo in
                    VStack(alignment: .leading, spacing: 3) {
                        Text(repo).font(.subheadline).foregroundStyle(Palette.text)
                        Text(line(for: repo))
                            .font(.caption).foregroundStyle(Palette.secondary)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
                }
                if installation.repos.isEmpty {
                    Text(L10n.tr("No repositories chosen for this installation."))
                        .font(.caption).foregroundStyle(Palette.tertiary)
                        .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
                }
            }
        }
    }

    /// Which bots a repository reaches, and on which branches: names from what the computer offers.
    private func line(for repo: String) -> String {
        guard let map = store.githubApp?.repos.first(where: { $0.repo == repo }) else { return L10n.tr("Not mapped") }
        let names = map.bots.map { store.bots[$0]?.name ?? $0 }.joined(separator: ", ")
        return map.branches.isEmpty ? names : "\(names) · \(map.branches.joined(separator: ", "))"
    }
}
