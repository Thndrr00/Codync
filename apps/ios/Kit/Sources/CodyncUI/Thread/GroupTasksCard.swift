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
            CappedHeight(max: 190) {
                ScrollView {
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
                            if task.id != rows.last?.id { Divider() }
                        }
                    }
                    .padding(10)
                }
                .scrollIndicators(.hidden)
                .scrollBounceBehavior(.basedOnSize)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Palette.surface, in: RoundedRectangle(cornerRadius: 18))
            .padding(.horizontal, 16)
            .transition(.move(edge: .bottom).combined(with: .opacity))
            .animation(Motion.reduced(Motion.layout, reduceMotion), value: rows.map(\.state))
        }
    }

    private func row(_ task: GroupTask) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 5) {
                Text(task.ownerName ?? L10n.tr("The lead"))
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(Palette.text)
                Text(stateLabel(task.state)).font(.caption2).foregroundStyle(Palette.tertiary)
                if !task.instruction.isEmpty {
                    Text(task.instruction).font(.caption2).foregroundStyle(Palette.tertiary).lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            HStack(spacing: 2) {
                if task.isStopped {
                    TaskPill("Continue", systemImage: "play.fill") { act(task, "continue") }
                    DropdownMenu(items: reassignItems(task)) {
                        HStack(spacing: 5) {
                            Image(systemName: "arrow.left.arrow.right").font(.system(size: 11, weight: .semibold))
                            Text(L10n.tr("Reassign")).font(.caption.weight(.semibold))
                        }
                        .foregroundStyle(Palette.secondary)
                        .padding(.horizontal, 9)
                        .padding(.vertical, 5)
                        .contentShape(.rect)
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
    private func reassignItems(_ task: GroupTask) -> () -> [MenuItem] {
        { group.members
            .filter { $0 != task.ownerId }
            .compactMap { model.bots[$0] }
            .map { member in MenuItem(member.name) { act(task, "reassign", owner: member.id) } } }
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
        Button(action: action) {
            HStack(spacing: 5) {
                Image(systemName: systemImage).font(.system(size: 11, weight: .semibold))
                Text(L10n.tr(title)).font(.caption.weight(.semibold))
            }
            .foregroundStyle(Palette.secondary)
            .padding(.horizontal, 9)
            .padding(.vertical, 5)
            .contentShape(.rect)
        }
        .buttonStyle(PressScale())
    }
}
