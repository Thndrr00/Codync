import CodyncKit
import SwiftUI

/// A channel's request tree over the composer, as the desktop `GroupTasks` and the Android `GroupTasksCard`: the
/// request, the members' assignments and their children, each with its state. Stop pauses a row and its branch; a
/// stopped row offers Continue, Reassign (another member) and Cancel.
struct GroupTasksCard: View {
    let group: Bot
    @Environment(BotStore.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var tasks: [GroupTask] { group.tasks.filter { $0.state != "cancelled" } }

    var body: some View {
        let rows = tasks
        if !rows.isEmpty {
            // Hugs its rows; scrolls only once they're taller than 190 points, so the composer stays on screen.
            CappedHeight(max: 190) {
                ViewThatFits(in: .vertical) {
                    tree(rows)
                    ScrollView { tree(rows) }.scrollIndicators(.hidden)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Palette.surface, in: RoundedRectangle(cornerRadius: 18))
            .padding(.horizontal, 16)
            .padding(.vertical, 4)
            .transition(.move(edge: .bottom).combined(with: .opacity))
            .animation(Motion.reduced(Motion.layout, reduceMotion), value: rows.map(\.state))
        }
    }

    private func tree(_ rows: [GroupTask]) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 5) {
                Image(systemName: "person.2").font(.system(size: 10, weight: .semibold))
                Text(L10n.tr("Waiting for sub-tasks")).font(.caption2)
                Spacer(minLength: 0)
            }
            .foregroundStyle(Palette.tertiary)
            .padding(.bottom, 4)
            ForEach(rows) { task in
                row(task)
                if task.id != rows.last?.id { Hairline() }
            }
        }
        .padding(10)
    }

    private func row(_ task: GroupTask) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 5) {
                Text(task.ownerName ?? L10n.tr("The lead"))
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(Palette.text)
                    .lineLimit(1)
                Text(stateLabel(task.state)).font(.caption2).foregroundStyle(Palette.tertiary).lineLimit(1)
                if !task.instruction.isEmpty {
                    Text(task.instruction).font(.caption2).foregroundStyle(Palette.tertiary).lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            .accessibilityElement(children: .combine)
            HStack(spacing: 2) {
                if task.isStopped {
                    TaskPill("Continue", systemImage: "play.fill") { act(task, "continue") }
                    let others = reassignable(task)
                    // A channel with nobody else to give it to offers no Reassign.
                    if !others.isEmpty {
                        DropdownMenu(items: { others.map { member in
                            MenuItem(member.name) { act(task, "reassign", owner: member.id) }
                        } }) {
                            TaskPillLabel("Reassign", systemImage: "arrow.left.arrow.right")
                        }
                    }
                    TaskPill("Cancel", systemImage: "xmark") { act(task, "cancel") }
                } else if task.state == "queued" || task.state == "running" {
                    TaskPill("Stop", systemImage: "stop.fill") { act(task, "stop") }
                }
                Spacer(minLength: 0)
            }
        }
        .padding(.vertical, 6)
    }

    /// The channel's other members, the one that already has the row left out.
    private func reassignable(_ task: GroupTask) -> [Bot] {
        model.members(of: group).filter { $0.id != task.ownerId }
    }

    private func act(_ task: GroupTask, _ action: String, owner: String? = nil) {
        model.groupTask(group.id, task: task.id, action: action, owner: owner)
    }

    private func stateLabel(_ state: String) -> String {
        switch state {
        case "queued": return L10n.tr("waiting")
        case "running": return L10n.tr("working")
        case "completed": return L10n.tr("done")
        case "failed": return L10n.tr("failed")
        case "paused": return L10n.tr("stopped")
        default: return L10n.tr("cancelled")
        }
    }
}

/// One named action on a sub-task.
private struct TaskPill: View {
    let title: String
    let systemImage: String
    let action: () -> Void

    init(_ title: String, systemImage: String, action: @escaping () -> Void) {
        self.title = title
        self.systemImage = systemImage
        self.action = action
    }

    var body: some View {
        Button(action: action) { TaskPillLabel(title, systemImage: systemImage) }
            .buttonStyle(PressScale())
    }
}

/// A sub-task action's look: an icon and its name, the same in its button and in the Reassign menu.
private struct TaskPillLabel: View {
    let title: String
    let systemImage: String

    init(_ title: String, systemImage: String) {
        self.title = title
        self.systemImage = systemImage
    }

    var body: some View {
        HStack(spacing: 5) {
            Image(systemName: systemImage).font(.system(size: 11, weight: .semibold))
            Text(L10n.tr(title)).font(.caption.weight(.semibold))
        }
        .foregroundStyle(Palette.secondary)
        .padding(.horizontal, 9)
        .padding(.vertical, 5)
        .contentShape(Rectangle())
    }
}
