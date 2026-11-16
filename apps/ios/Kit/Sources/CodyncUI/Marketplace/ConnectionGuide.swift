import CodyncKit
import SwiftUI

/// How to connect an app (the host's `guide`, host/src/market/guide): what to do at the vendor, and
/// its own page to do it on. Nothing is drawn for an app the host sent no guide for.
struct ConnectionGuideCard: View {
    let guide: ConnectionGuide?
    let name: String
    @Environment(BotStore.self) private var model
    @Environment(\.openURL) private var openURL

    var body: some View {
        if let guide, !guide.steps.isEmpty {
            CardSection("How to connect", footer: footer(guide)) {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(Array(guide.steps.enumerated()), id: \.offset) { index, step in
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text("\(index + 1).")
                                .font(.footnote.monospacedDigit())
                                .foregroundStyle(Palette.tertiary)
                            Text(step)
                                .font(.callout)
                                .foregroundStyle(Palette.text)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    if let url = guide.url.flatMap(URL.init(string:)) {
                        Button(L10n.fill(L10n.tr("Open {name}’s page"), ["name": name]), systemImage: "safari") {
                            openURL(url)
                        }
                        .buttonStyle(.secondary)
                    }
                }
                .padding(.vertical, 2)
            }
        }
    }

    private func footer(_ guide: ConnectionGuide) -> String {
        guide.signingIn
            ? L10n.tr("The sign-in stays on {name}’s own page.").replacingOccurrences(of: "{name}", with: name)
            : L10n.tr("Keys are saved on {name} only.").replacingOccurrences(of: "{name}", with: model.hostName)
    }
}

/// The key note a row carries before anybody adds the app.
struct KeyNote: View {
    let text: String

    var body: some View {
        Label(L10n.tr(text), systemImage: "key")
            .labelStyle(.titleAndIcon)
            .font(.caption)
            .foregroundStyle(Palette.tertiary)
            .lineLimit(1)
    }
}
