import CodyncKit
import SwiftUI
import UniformTypeIdentifiers

/// A brain's file, waiting for the share sheet.
private struct BrainFile: Identifiable {
    let id = UUID()
    let url: URL
}

/// The brains a bot can work in: the Fleet one (the memory all bots used to share) and the projects, to pick one, make
/// one, rename it, delete it, or move a brain to another computer.
struct BrainSheet: View {
    let memory: MemoryStore
    @Environment(\.dismissModal) private var dismiss
    @State private var name = ""
    @State private var creating = false
    @State private var renaming: Brain?
    @State private var deleting: Brain?
    @State private var importing = false
    @State private var shared: BrainFile?
    @State private var notice: String?

    private var brains: [Brain] { memory.brains ?? [] }
    private var current: Brain? { memory.view?.brain }
    private var trimmed: String { name.trimmingCharacters(in: .whitespacesAndNewlines) }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Brain") {
                IconButton("Close", systemImage: "xmark") { dismiss() }
            }
            CardForm {
                CardSection("A brain is memory bots share: a project's, or the Fleet one, which every bot that uses the shared memory reads.") {
                    ChoiceList(selection: Binding(get: { current?.id ?? "" }, set: { pick in
                        guard let brain = brains.first(where: { $0.id == pick }), brain.id != current?.id else { return }
                        Task { await memory.joinBrain(brain) }
                    }), options: brains.map { brain in
                        (id: brain.id, label: brain.shared ? "Fleet (every bot)" : brain.name, detail: brain.shared
                            ? "What your bots share. Joining keeps the memory you already share."
                            : L10n.tr("{n} bots").replacingOccurrences(of: "{n}", with: "\(brain.members?.count ?? 0)"))
                    })
                    if current != nil {
                        Button("Leave this brain", action: { Task { await memory.leaveBrain() } })
                            .buttonStyle(.secondary)
                    }
                }
                CardSection("New brain") {
                    Button("Create a brain") { name = ""; creating = true }
                        .buttonStyle(.secondary)
                }
                if !brains.isEmpty {
                    CardSection("Projects") {
                        ForEach(brains.filter { !$0.shared }) { brain in
                            HStack(spacing: 8) {
                                Text(brain.name).frame(maxWidth: .infinity, alignment: .leading)
                                IconButton("Rename \(brain.name)", systemImage: "pencil") { name = brain.name; renaming = brain }
                                IconButton("Save \(brain.name) to a file", systemImage: "square.and.arrow.up") {
                                    Task {
                                        guard let url = await memory.exportBrain(brain) else { return }
                                        shared = BrainFile(url: url)
                                    }
                                }
                                IconButton("Delete \(brain.name)", systemImage: "trash") { deleting = brain }
                            }
                        }
                        Text("Deleting a brain keeps its bots and their own memory; what the brain held goes.")
                            .font(.caption).foregroundStyle(Palette.secondary)
                    }
                }
                CardSection("Import a brain") {
                    Button("Import from a file") { importing = true }
                        .buttonStyle(.secondary)
                    if let notice {
                        Text(notice).font(.caption).foregroundStyle(Palette.secondary)
                    }
                }
                if let error = memory.error {
                    Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                }
            }
        }
        .background(Palette.background)
        .task { await memory.loadBrains() }
        .codyncDialog("New brain", isPresented: $creating, message: "What the bots in it share.", field: DialogField("Project name", text: $name)) {
            [DialogAction("Create") {
                guard !trimmed.isEmpty else { return }
                Task { await memory.saveBrain(trimmed, id: nil) }
            }]
        }
        .codyncDialog(renaming.map { L10n.tr("Rename {name}").replacingOccurrences(of: "{name}", with: $0.name) } ?? "Rename",
                      isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } }),
                      field: DialogField("Brain name", text: $name)) {
            [DialogAction("Rename") {
                guard let brain = renaming, !trimmed.isEmpty else { return }
                Task { await memory.saveBrain(trimmed, id: brain.id) }
            }]
        }
        .codyncDialog(deleting.map { L10n.tr("Delete the brain {name}?").replacingOccurrences(of: "{name}", with: $0.name) } ?? "Delete",
                      isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } }),
                      message: "Its bots go back to the Fleet brain and keep their own memory. What the brain held is gone.") {
            [DialogAction("Delete", destructive: true) {
                guard let brain = deleting else { return }
                Task { await memory.deleteBrain(brain) }
            }]
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.json]) { result in
            Task {
                guard let data = try? Data(contentsOf: try result.get()),
                      let skipped = await memory.importBrain(data, name: nil) else { return }
                notice = skipped > 0
                    ? L10n.tr("Imported, {n} files were turned away.").replacingOccurrences(of: "{n}", with: "\(skipped)")
                    : L10n.tr("Imported.")
            }
        }
        .codyncSheet(item: $shared) { file in
            FileExportSheet(url: file.url) { shared = nil }
        }
    }
}
