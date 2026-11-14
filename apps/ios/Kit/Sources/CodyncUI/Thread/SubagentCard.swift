import CodyncKit
import SwiftUI

/// The subagents one turn started: a header with their progress and time, and a row for each with
/// its task, what it does now and how long it has taken. The list is open while one works.
struct SubagentCard: View {
    let entry: Entry
    @State private var choice: Bool?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var subagents: [EntrySubagent] { entry.data.subagents ?? [] }
    private var running: Int { subagents.filter(\.isRunning).count }
    private var open: Bool { choice ?? running > 0 }

    var body: some View {
        if !subagents.isEmpty {
            VStack(alignment: .leading, spacing: 4) {
                Button {
                    withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { choice = !open }
                } label: {
                    HStack(spacing: 8) {
                        Text(L10n.tr("Subagents"))
                            .font(.subheadline.weight(.medium))
                            .foregroundStyle(Palette.text)
                        SubagentProgress(subagents: subagents)
                        if subagents.contains(where: { $0.shownState == "failed" }) {
                            Text(L10n.tr("Failed"))
                                .font(.caption2)
                                .foregroundStyle(Palette.danger)
                        }
                        TimelineView(.periodic(from: .now, by: 1)) { context in
                            Text(subagentDuration(subagents, now: context.date))
                                .font(.caption2.monospacedDigit())
                                .foregroundStyle(Palette.tertiary)
                        }
                        Image(systemName: open ? "chevron.down" : "chevron.right")
                            .font(.system(size: 10, weight: .semibold))
                            .foregroundStyle(Palette.tertiary)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel(L10n.tr("Subagents"))
                .accessibilityValue("\(subagents.count - running) \(L10n.tr("Done")), \(running) \(L10n.tr("Working"))")
                if open {
                    VStack(alignment: .leading, spacing: 6) {
                        ForEach(subagents) { subagent in
                            SubagentRow(subagent: subagent)
                        }
                    }
                    .padding(.leading, 4)
                    .transition(.opacity)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.trailing, 40)
        }
    }
}

/// The header of a subagent card the reader scrolled away from while its subagents work. It floats
/// under the chat header and takes no height from the chat; a tap goes back to the card.
struct SubagentDock: View {
    let entry: Entry
    let reveal: () -> Void

    private var subagents: [EntrySubagent] { entry.data.subagents ?? [] }
    private var current: EntrySubagent? { subagents.first(where: \.isRunning) }

    var body: some View {
        Button(action: reveal) {
            HStack(spacing: 8) {
                Text(L10n.tr("Subagents"))
                    .font(.footnote.weight(.medium))
                    .foregroundStyle(Palette.text)
                SubagentProgress(subagents: subagents)
                if let current {
                    HStack(spacing: 4) {
                        Image(systemName: "circle.dotted")
                            .font(.system(size: 10))
                            .foregroundStyle(Palette.accent)
                        Text(current.step.isEmpty ? L10n.tr("Working…") : current.step)
                            .font(.caption2)
                            .foregroundStyle(Palette.secondary)
                            .lineLimit(1)
                            .truncationMode(.tail)
                    }
                    .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
                }
                TimelineView(.periodic(from: .now, by: 1)) { context in
                    Text(subagentDuration(subagents, now: context.date))
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(Palette.tertiary)
                }
                Image(systemName: "chevron.right")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(Palette.tertiary)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Palette.surface, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .contentShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        }
        .buttonStyle(PressScale())
        .padding(.horizontal, 16)
        .padding(.top, 4)
        .accessibilityLabel(L10n.tr("Go to the subagents"))
    }
}

/// One subagent's row: its state, task and newest step, and how long it has taken.
private struct SubagentRow: View {
    let subagent: EntrySubagent

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: symbol)
                .font(.system(size: 12))
                .foregroundStyle(tint)
                .frame(width: 14)
                .padding(.top, 2)
                .accessibilityLabel(L10n.tr(stateLabel))
            VStack(alignment: .leading, spacing: 1) {
                Text(subagent.task.isEmpty ? L10n.tr("Subagents") : subagent.task)
                    .font(.footnote)
                    .foregroundStyle(Palette.text)
                    .lineLimit(1)
                Text(step)
                    .font(.caption2)
                    .foregroundStyle(subagent.shownState == "failed" ? Palette.danger : Palette.secondary)
                    .lineLimit(1)
            }
            .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
            TimelineView(.periodic(from: .now, by: 1)) { context in
                Text(subagentDuration(subagent, now: context.date))
                    .font(.caption2.monospacedDigit())
                    .foregroundStyle(Palette.tertiary)
            }
        }
        .padding(.vertical, 1)
    }

    private var symbol: String {
        switch subagent.shownState {
        case "done": "checkmark.circle.fill"
        case "failed": "xmark.circle.fill"
        case "stopped": "minus.circle"
        default: "circle.dotted"
        }
    }

    private var tint: Color {
        switch subagent.shownState {
        case "failed": Palette.danger
        case "running": Palette.accent
        default: Palette.secondary
        }
    }

    private var stateLabel: String {
        switch subagent.shownState {
        case "done": "Done"
        case "failed": "Failed"
        case "stopped": "Stopped"
        default: "Working"
        }
    }

    private var step: String {
        if !subagent.step.isEmpty { return subagent.step }
        switch subagent.shownState {
        case "done": return L10n.tr("Done")
        case "failed": return L10n.tr("Failed")
        case "stopped": return L10n.tr("Stopped")
        default: return L10n.tr("Working…")
        }
    }
}

/// A ring of the subagents that finished, and "{finished} of {total}".
private struct SubagentProgress: View {
    let subagents: [EntrySubagent]

    var body: some View {
        let finished = subagents.filter { !$0.isRunning }.count
        let total = max(1, subagents.count)
        HStack(spacing: 4) {
            ZStack {
                Circle()
                    .stroke(Palette.tertiary.opacity(0.3), lineWidth: 2.5)
                Circle()
                    .trim(from: 0, to: Double(finished) / Double(total))
                    .stroke(Palette.accent, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
            .frame(width: 14, height: 14)
            Text(L10n.tr("{finished} of {total}")
                .replacingOccurrences(of: "{finished}", with: String(finished))
                .replacingOccurrences(of: "{total}", with: String(subagents.count)))
                .font(.caption2.monospacedDigit())
                .foregroundStyle(Palette.secondary)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(L10n.tr("{finished} of {total}")
            .replacingOccurrences(of: "{finished}", with: String(finished))
            .replacingOccurrences(of: "{total}", with: String(subagents.count)))
    }
}

/// "12s" or "1m 05s": how long a subagent, or all of them, took.
func subagentDuration(_ subagents: [EntrySubagent], now: Date) -> String {
    let start = subagents.map(\.startedAt).min() ?? 0
    let running = subagents.contains(where: \.isRunning)
    let end = running ? Int64(now.timeIntervalSince1970 * 1000) : (subagents.compactMap(\.endedAt).max() ?? start)
    return subagentDuration(end - start)
}

func subagentDuration(_ subagent: EntrySubagent, now: Date) -> String {
    let end = subagent.endedAt ?? Int64(now.timeIntervalSince1970 * 1000)
    return subagentDuration(end - subagent.startedAt)
}

private func subagentDuration(_ ms: Int64) -> String {
    let total = max(0, Int(ms / 1000))
    if total < 60 { return "\(total)s" }
    return "\(total / 60)m \(String(format: "%02d", total % 60))s"
}
