import CodyncKit
import SwiftUI

/// A bot's missions: the objective that outlives a turn, its steps and how far it got. Same actions as
/// the desktop inspector and the terminal UI, laid out for a phone.
struct MissionsView: View {
    let botId: String
    @Environment(BotStore.self) private var host
    @State private var store: MissionsStore?
    @State private var composing = false
    @State private var abandoning: Mission?
    @State private var reason = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if let store {
                header(store)
                if let error = store.error {
                    Text(error).font(.footnote).foregroundStyle(Palette.danger).textSelection(.enabled)
                }
                if store.loading && store.missions.isEmpty {
                    Spinner()
                } else if store.open.isEmpty {
                    Text(L10n.tr("No open missions.")).foregroundStyle(Palette.secondary)
                }
                ForEach(store.open) { mission in
                    MissionRow(mission: mission, store: store) { asking in
                        reason = ""
                        abandoning = asking
                    }
                }
                if !store.finished.isEmpty { finished(store) }
            } else {
                Text(L10n.tr("Loading missions…")).foregroundStyle(Palette.secondary)
            }
        }
        .task(id: botId) {
            let store = MissionsStore(botId: botId) { try await host.ready() }
            self.store = store
            await store.load()
        }
        .codyncSheet(isPresented: $composing) {
            if let store { NewMissionView(store: store) }
        }
        .codyncDialog(
            L10n.tr("Abandon mission"),
            isPresented: Binding(get: { abandoning != nil }, set: { if !$0 { abandoning = nil } }),
            message: abandoning?.title,
            field: DialogField(L10n.tr("Why it stopped"), text: $reason)
        ) {
            let target = abandoning
            return [DialogAction(L10n.tr("Abandon"), destructive: true) {
                guard let target, let store, !reason.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
                Task { await store.move(target, to: .abandoned, stopReason: reason) }
            }]
        }
    }

    private func header(_ store: MissionsStore) -> some View {
        HStack {
            Text(L10n.tr("Missions")).font(.headline)
            Spacer()
            IconButton(L10n.tr("New mission"), systemImage: "plus") {
                withAnimation(Motion.layout) { composing = true }
            }.disabled(store.busy)
        }
    }

    private func finished(_ store: MissionsStore) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(L10n.tr("Finished")).font(.headline)
                Spacer()
                IconButton(store.showFinished ? L10n.tr("Hide finished") : L10n.tr("Show finished"),
                           systemImage: store.showFinished ? "chevron.up" : "chevron.down") {
                    withAnimation(Motion.layout) { store.showFinished.toggle() }
                    Task { await store.load() }
                }
            }
            if store.showFinished {
                ForEach(store.finished) { mission in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 8) {
                            MissionPill(status: mission.status)
                            Text(mission.title).font(.subheadline.weight(.semibold))
                        }
                        if let stopped = mission.stopReason {
                            Text(stopped).font(.caption).foregroundStyle(Palette.secondary)
                        }
                        HStack(spacing: 8) {
                            Button(L10n.tr("Reopen")) { Task { await store.move(mission, to: .active) } }
                                .buttonStyle(.secondary).disabled(store.busy)
                            Button(L10n.tr("Abandon")) {
                                reason = ""
                                abandoning = mission
                            }.buttonStyle(.secondary).disabled(store.busy)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .outlinedCard()
                }
            } else {
                Text(L10n.tr("Finished missions are hidden."))
                    .font(.footnote).foregroundStyle(Palette.secondary)
            }
        }
    }
}

/// One mission: its status, goal, steps and checkpoint, and what can be done with it.
private struct MissionRow: View {
    let mission: Mission
    let store: MissionsStore
    let askAbandon: (Mission) -> Void
    @State private var checkpoint = ""
    @State private var draft = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                MissionPill(status: mission.status)
                Text(mission.title).font(.subheadline.weight(.semibold))
                Spacer(minLength: 0)
                actions
            }
            if !mission.goal.isEmpty && mission.goal != mission.title {
                Text(mission.goal).font(.caption).foregroundStyle(Palette.secondary)
            }
            if let stopped = mission.stopReason {
                Text(stopped).font(.caption).foregroundStyle(Palette.secondary)
            }
            ForEach(mission.steps) { step in steps(step) }
            HStack(spacing: 8) {
                TextField(L10n.tr("Add a step"), text: $draft)
                    .textFieldStyle(.plain)
                    .onSubmit { add() }
                IconButton(L10n.tr("Add step"), systemImage: "plus", action: add)
                    .disabled(store.busy || draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            HStack(spacing: 8) {
                TextField(L10n.tr("What is done and what is left"), text: $checkpoint, axis: .vertical)
                    .lineLimit(2...4)
                    .textFieldStyle(.plain)
                IconButton(L10n.tr("Save checkpoint"), systemImage: "square.and.arrow.down") {
                    Task { await store.checkpoint(mission, checkpoint) }
                }.disabled(store.busy || checkpoint == mission.checkpoint)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .outlinedCard()
        .onAppear { checkpoint = mission.checkpoint }
        .onChange(of: mission.checkpoint) { _, fresh in checkpoint = fresh }
    }

    private var actions: some View {
        HStack(spacing: 2) {
            if let next = mission.status.next {
                IconButton(next.nextTitle, systemImage: icon) {
                    Task { await store.move(mission, to: next) }
                }.disabled(store.busy)
            }
            IconButton(L10n.tr("Finish"), systemImage: "checkmark.seal") {
                Task { await store.move(mission, to: .done) }
            }.disabled(store.busy)
            IconButton(L10n.tr("Abandon"), systemImage: "xmark.circle") { askAbandon(mission) }
                .disabled(store.busy)
        }
    }

    /// The glyph of the next move: pause, resume, or back into the list of open ones.
    private var icon: String {
        switch mission.status {
        case .active: "pause"
        case .done, .abandoned: "arrow.uturn.backward"
        case .proposed, .waiting: "play"
        }
    }

    private func steps(_ step: MissionStep) -> some View {
        HStack(spacing: 8) {
            Text(step.status.mark).font(.caption).foregroundStyle(Palette.secondary).frame(width: 14)
            Text(step.text)
                .font(.caption)
                .strikethrough(step.status == .done)
                .foregroundStyle(step.status == .done || step.status == .skipped ? Palette.secondary : Palette.text)
            Spacer(minLength: 0)
            if step.status != .done {
                IconButton(L10n.tr("Start step"), systemImage: "play") {
                    Task { await store.setStep(mission, step, to: .active) }
                }.disabled(store.busy || step.status == .active)
                IconButton(L10n.tr("Mark done"), systemImage: "checkmark.circle") {
                    Task { await store.setStep(mission, step, to: .done) }
                }.disabled(store.busy)
                IconButton(L10n.tr("Skip step"), systemImage: "forward") {
                    Task { await store.setStep(mission, step, to: .skipped) }
                }.disabled(store.busy)
            } else {
                IconButton(L10n.tr("Reopen step"), systemImage: "arrow.uturn.backward") {
                    Task { await store.setStep(mission, step, to: .pending) }
                }.disabled(store.busy)
            }
        }
    }

    private func add() {
        let text = draft
        draft = ""
        Task { await store.addStep(mission, text) }
    }
}

/// A mission's status as a small capsule.
private struct MissionPill: View {
    let status: MissionStatus

    var body: some View {
        Text(status.label)
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(color)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(fill, in: Capsule())
    }

    private var color: Color {
        switch status {
        case .active: Palette.accent
        case .waiting: Palette.attention
        case .done: Palette.added
        case .proposed, .abandoned: Palette.secondary
        }
    }

    private var fill: Color {
        switch status {
        case .active, .waiting, .done: color.opacity(0.16)
        case .proposed, .abandoned: Palette.surface
        }
    }
}

/// The new-mission sheet: what to reach, why, and the steps somebody already knows.
private struct NewMissionView: View {
    let store: MissionsStore
    @Environment(\.dismissModal) private var dismiss
    @State private var title = ""
    @State private var goal = ""
    @State private var steps = ""

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(L10n.tr("New mission")) {
                if store.busy { Spinner() }
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    TextField(L10n.tr("What is the objective?"), text: $title)
                        .font(.title3.weight(.semibold)).textFieldStyle(.plain)
                    TextField(L10n.tr("Why, and what counts as done"), text: $goal, axis: .vertical)
                        .lineLimit(2...5).textFieldStyle(.plain)
                    TextField(L10n.tr("One step per line, if you know them"), text: $steps, axis: .vertical)
                        .lineLimit(3...10).textFieldStyle(.plain)
                    Button(L10n.tr("Create mission")) {
                        let list = steps.split(separator: "\n")
                            .map { $0.trimmingCharacters(in: .whitespaces) }
                            .filter { !$0.isEmpty }
                        let goal = goal
                        let title = title
                        Task {
                            await store.create(title: title, goal: goal, steps: list)
                            dismiss()
                        }
                    }
                    .buttonStyle(.primary)
                    .disabled(store.busy || title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .background(Palette.background)
    }
}
