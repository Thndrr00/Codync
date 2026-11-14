import CodyncKit
import CodyncUI
import Foundation
import SwiftUI

/// Usage limits (opened from the State tab): the computer's health, then a Breakdown with its limits or its skills.
struct UsageView: View {
    @Environment(BotStore.self) private var model
    @State private var collapsed: Set<String> = []
    @State private var week: WeekSummary?
    @State private var tab: Tab = .limits

    private enum Tab: Hashable { case limits, skills }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if let health = model.health { HealthCard(health: health) }
                VStack(spacing: 8) {
                    Text(L10n.tr("Breakdown")).font(.caption).foregroundStyle(Palette.tertiary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    TabChoice(selection: $tab, options: [(.limits, "Limits"), (.skills, "Skills")])
                }
                if tab == .limits {
                    ForEach(model.usage.providers) { provider in
                        ProviderCard(provider: provider, collapsed: Binding(
                            get: { collapsed.contains(provider.id) },
                            set: { if $0 { collapsed.insert(provider.id) } else { collapsed.remove(provider.id) } }
                        ))
                    }
                    if let week, week.activeBots > 0 { WeekCard(week: week) }
                    if model.usage.providers.isEmpty {
                        VStack(spacing: 8) {
                            Image(systemName: "chart.bar").font(.system(size: 40)).foregroundStyle(Palette.tertiary)
                            Text(L10n.tr("No usage yet")).font(.title3.weight(.semibold)).foregroundStyle(Palette.text)
                            Text(L10n.tr("Your computer reads plan limits from the agents installed on it."))
                                .font(.subheadline).foregroundStyle(Palette.secondary)
                        }
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: .infinity)
                        .padding(.top, 40)
                    }
                } else {
                    SkillCard(usage: model.skillUsage)
                }
            }
            .padding(16)
        }
        .background(Palette.background)
        .refreshable {
            await model.refreshUsage()
            await model.refreshSkillUsage()
            await model.refreshHealth()
        }
        .task {
            week = await model.weekSummary()
            await model.refreshSkillUsage()
            await model.refreshHealth()
        }
    }
}

/// The computer's RAM, CPU and disk, read from `hostHealth`.
private struct HealthCard: View {
    let health: HostHealth

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(health.lines.indices, id: \.self) { index in
                let line = health.lines[index]
                HStack {
                    Text(L10n.tr(line.label)).font(.footnote).foregroundStyle(Palette.secondary)
                    Spacer()
                    Text(line.value).font(.footnote.monospacedDigit()).foregroundStyle(Palette.text)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(Palette.surface, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }
}

/// How often each skill's text entered a turn, with its estimated tokens, cost and share.
private struct SkillCard: View {
    let usage: SkillUsage?

    var body: some View {
        let rows = usage?.skills ?? []
        VStack(alignment: .leading, spacing: 12) {
            if rows.isEmpty {
                Text(L10n.tr("No skill has been loaded yet.")).font(.subheadline).foregroundStyle(Palette.secondary)
            } else {
                ForEach(rows) { row in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(row.name).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text).lineLimit(1)
                            Spacer()
                            Text(costText(row.costUsd)).font(.footnote.monospacedDigit())
                                .foregroundStyle(row.costUsd == nil ? Palette.secondary : Palette.text)
                        }
                        Text(detail(row)).font(.caption).foregroundStyle(Palette.secondary)
                    }
                    .padding(.vertical, 2)
                }
            }
            Text(L10n.tr("A load is counted when an agent reads a skill's SKILL.md during a turn."))
                .font(.caption2).foregroundStyle(Palette.tertiary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(Palette.surface, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }

    private func detail(_ row: SkillUsageRow) -> String {
        let loads = "\(row.loads) \(L10n.tr("loads"))"
        let tokens = "\(row.tokens.formatted()) \(L10n.tr("tokens"))"
        let share = "\(Int((row.share * 100).rounded()))%"
        let agents = "\(row.agents) \(L10n.tr("agents"))"
        return [loads, tokens, share, agents].joined(separator: " · ")
    }

    private func costText(_ cost: Double?) -> String {
        guard let cost else { return L10n.tr("Unavailable") }
        if cost == 0 { return "$0.00" }
        return cost < 0.01 ? L10n.tr("Under $0.01") : String(format: "$%.2f", cost)
    }
}

/// The last seven days, as counts: what was said, done and approved.
private struct WeekCard: View {
    let week: WeekSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(L10n.tr("Your week")).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
            row("Messages you sent", "\(week.messages)")
            row("Replies from your bots", "\(week.replies)")
            row("Tool calls", "\(week.tools)")
            row("File edits", "\(week.fileEdits)")
            row("Approvals answered", "\(week.approvalsAnswered) / \(week.approvalsAsked)")
            if week.approvalsWaiting > 0 { row("Approvals still waiting", "\(week.approvalsWaiting)") }
            if !week.topBots.isEmpty {
                Text("\(L10n.tr("Busiest")): \(week.topBots.map { $0.name.isEmpty ? L10n.tr("Bot") : $0.name }.joined(separator: ", "))")
                    .font(.caption).foregroundStyle(Palette.tertiary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(Palette.surface, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }

    private func row(_ label: String, _ value: String) -> some View {
        HStack {
            Text(L10n.tr(label)).font(.footnote).foregroundStyle(Palette.secondary)
            Spacer()
            Text(value).font(.footnote.monospacedDigit()).foregroundStyle(Palette.text)
        }
    }
}

private struct ProviderCard: View {
    let provider: UsageProvider
    @Binding var collapsed: Bool
    @AppStorage(SharedStore.usageIconStyleKey, store: UserDefaults(suiteName: SharedStore.appGroup))
    private var usageIconStyle = UsageIconStyle.character.rawValue
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                ProviderMascot(provider, size: 22, style: UsageIconStyle(rawValue: usageIconStyle) ?? .character)
                    .frame(width: 34, height: 34)
                    .background(provider.tint.opacity(0.12), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                Text(provider.name).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                Spacer()
                IconButton(collapsed ? "Expand" : "Collapse", systemImage: "chevron.up") {
                    withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { collapsed.toggle() }
                }
                .rotationEffect(.degrees(collapsed ? 180 : 0))
            }

            if let status = provider.statusLabel {
                Text(L10n.tr(status)).font(.caption).foregroundStyle(Palette.secondary)
            }
            if let top = provider.tightest {
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Text(L10n.tr(top.title))
                        Spacer()
                        Text(top.usedText).monospacedDigit()
                    }
                    .font(.subheadline)
                    .foregroundStyle(Palette.text)
                    UsageTicks(percent: top.percent, tint: provider.widgetTint, height: 15)
                    if let reset = top.resetsShort() {
                        Text(L10n.resetLine(reset)).font(.caption.monospacedDigit()).foregroundStyle(Palette.tertiary)
                            .frame(maxWidth: .infinity, alignment: .trailing)
                    }
                }
            }

            if !collapsed {
                VStack(spacing: 8) {
                    ForEach(provider.windows) { w in
                        row(L10n.tr(w.title), w.reading)
                        if let reset = w.resetsShort() { row(L10n.tr("Resets"), L10n.resetLine(reset)) }
                    }
                    row("Source", source)
                    row("Updated", Date(milliseconds: provider.updatedAt).formatted(date: .abbreviated, time: .shortened))
                }
                .transition(.opacity)
            }
        }
        .padding(14)
        .background(Palette.surface, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }

    private var source: String {
        switch provider.source {
        case "claude": "Claude Code"
        case "statusline": "Claude Code status line"
        case "agent": "A running bot"
        case "sessions": "Codex sessions"
        case "antigravity": "Antigravity quota report"
        default: provider.source
        }
    }

    private func row(_ label: String, _ value: String) -> some View {
        HStack {
            Text(L10n.tr(label)).foregroundStyle(Palette.secondary)
            Spacer()
            Text(value).foregroundStyle(Palette.text).monospacedDigit()
        }
        .font(.footnote)
    }
}
