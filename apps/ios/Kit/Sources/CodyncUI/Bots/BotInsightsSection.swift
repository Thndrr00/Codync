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
                    if !insights.description.isEmpty {
                        Text(insights.description).font(.footnote).foregroundStyle(Palette.secondary)
                    }
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
                    let week = insights.week
                    Text(week.verdict).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(week.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Steps")): \(week.totalLabel)")
                        Text("\(L10n.tr("Trend")): \(week.trend.label)")
                    }
                    .font(.footnote).foregroundStyle(Palette.secondary)
                    if let peak = week.peak {
                        Text("\(L10n.tr("Busiest")): \(peak.label) · \(peak.count)")
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(week.top, id: \.label) { day in
                        HStack(spacing: 6) {
                            Text(day.label).font(.footnote).foregroundStyle(Palette.tertiary)
                            Text(day.bar).font(.footnote).foregroundStyle(Palette.accent)
                            Text("\(day.count)").font(.footnote).foregroundStyle(Palette.secondary)
                        }
                    }
                    let profile = insights.profile
                    Text(profile.bar).font(.footnote).foregroundStyle(Palette.accent)
                    Text(profile.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    if !profile.nextUp.isEmpty {
                        Text("\(L10n.tr("Next up")): \(profile.nextUp.map(\.title).joined(separator: " · "))")
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(profile.caps) { cap in
                        Text("\(cap.enabled ? "●" : "○") \(cap.title): \(cap.detail)")
                            .font(.footnote)
                            .foregroundStyle(cap.enabled ? Palette.text : Palette.secondary)
                    }
                    let health = insights.health
                    Text(health.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(health.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(health.checks) { item in
                        Text("\(item.title): \(item.detail)")
                            .font(.footnote)
                            .foregroundStyle(item.severity == "error" ? Palette.danger : Palette.secondary)
                    }
                    let activity = insights.activity
                    Text(activity.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Steps")): \(activity.steps)")
                        if activity.errors > 0 {
                            Text("\(activity.errors) \(L10n.tr("failed"))").foregroundStyle(Palette.danger)
                        }
                    }
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
                    if !activity.groups.isEmpty {
                        Text(activity.groups.prefix(6).map { "\($0.count) \($0.label)" }.joined(separator: " · "))
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    if !activity.days.isEmpty {
                        ForEach(activity.days.suffix(5)) { day in
                            HStack(spacing: 6) {
                                Text(day.label).font(.footnote).foregroundStyle(Palette.tertiary)
                                Text(String(repeating: "▮", count: max(1, day.width)))
                                    .font(.footnote).foregroundStyle(Palette.accent)
                                Text("\(day.count)").font(.footnote).foregroundStyle(Palette.secondary)
                            }
                        }
                    }
                    ForEach(activity.items.prefix(6), id: \.text) { step in
                        VStack(alignment: .leading, spacing: 1) {
                            Text(step.text)
                                .font(.footnote)
                                .foregroundStyle(step.status == "error" ? Palette.danger : Palette.text)
                            if !step.detail.isEmpty {
                                Text(step.detail).font(.footnote).foregroundStyle(Palette.tertiary)
                            }
                        }
                    }
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
