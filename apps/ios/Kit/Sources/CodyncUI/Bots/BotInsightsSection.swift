import CodyncKit
import SwiftUI

/// The bot's routines, model and daily use, read from the host's `botInsights` method.
struct BotInsightsSection: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var insights: BotInsights?
    @State private var failed = false

    var body: some View {
        CardSection(L10n.tr("Insights")) {
            if failed {
                Text(L10n.tr("Could not load insights.")).font(.footnote).foregroundStyle(Palette.secondary)
            } else if let insights {
                VStack(alignment: .leading, spacing: 10) {
                    if let provider = insights.provider {
                        Label {
                            Text(provider).font(.subheadline).foregroundStyle(Palette.text)
                        } icon: {
                            Image(systemName: "cpu").foregroundStyle(Palette.secondary)
                        }
                    }
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Tokens")): \(insights.usage.usedText)")
                        Text("\(L10n.tr("Skills")): \(insights.skills.total)")
                    }
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
                    if !insights.recentWork.isEmpty {
                        Text(L10n.tr("Recent work")).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                        ForEach(insights.recentWork, id: \.self) { line in
                            Text(line).font(.footnote).foregroundStyle(Palette.secondary)
                        }
                    }
                    if insights.routines.isEmpty {
                        Text(L10n.tr("No routines yet.")).font(.footnote).foregroundStyle(Palette.tertiary)
                    } else {
                        ForEach(insights.routines) { routine in
                            VStack(alignment: .leading, spacing: 2) {
                                HStack(spacing: 6) {
                                    Text(routine.name).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                                    if !routine.enabled {
                                        Text(L10n.tr("Paused")).font(.footnote).foregroundStyle(Palette.secondary)
                                    }
                                }
                                if !routine.schedules.isEmpty {
                                    Text(routine.schedules.joined(separator: " · "))
                                        .font(.footnote).foregroundStyle(Palette.secondary)
                                }
                                if let next = routine.nextIn {
                                    Text(L10n.tr("Next in {time}").replacingOccurrences(of: "{time}", with: next))
                                        .font(.footnote).foregroundStyle(Palette.tertiary)
                                }
                            }
                        }
                    }
                }
            } else {
                Text(L10n.tr("Loading…")).font(.footnote).foregroundStyle(Palette.secondary)
            }
        }
        .task(id: botId) { await load() }
    }

    private func load() async {
        insights = nil
        failed = false
        guard let client = model.client else { return }
        do {
            insights = try await client.call("botInsights", ["botId": botId])
        } catch {
            failed = true
        }
    }
}
