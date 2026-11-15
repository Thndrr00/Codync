import CodyncKit
import SwiftUI
import UniformTypeIdentifiers

/// Extensions › Modules: what a person added to Codync beyond what it ships with. Each one is a signed folder the
/// host runs in its own process; nothing it offers a bot runs until the owner approves it here.
struct ModulesTab: View {
    @Environment(BotStore.self) private var model
    @State private var modules: [HostModule] = []
    @State private var loaded = false
    @State private var error: String?
    @State private var busy: String?
    @State private var open: HostModule?
    @State private var importing = false
    @State private var notice: String?
    let query: String

    private var shown: [HostModule] {
        guard !query.isEmpty else { return modules }
        return modules.filter {
            $0.name.localizedCaseInsensitiveContains(query) || $0.description.localizedCaseInsensitiveContains(query)
        }
    }

    var body: some View {
        CardForm {
            if let error {
                Text(L10n.tr(error)).font(.footnote).foregroundStyle(Palette.danger)
            }
            CardSection("On this computer") {
                if loaded && shown.isEmpty {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(L10n.tr("No modules yet")).foregroundStyle(Palette.text)
                        Text(L10n.tr("A module adds tools and a screen to Codync, signed by whoever made it."))
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                }
                ForEach(shown) { module in
                    ModuleRow(module: module) { open = module }
                }
            }
            CardSection(footer: notice) {
                Button("Install from a file") { importing = true }
                    .buttonStyle(.secondary)
            }
        }
        .task { await load() }
        .codyncSheet(item: $open) { module in
            ModuleSheet(module: module, apply: { modules = $0 })
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.json]) { result in
            Task {
                guard let data = try? Data(contentsOf: try result.get()),
                      let bundle = try? JSONDecoder().decode(JSONValue.self, from: data) else {
                    notice = L10n.tr("That file is not a module bundle.")
                    return
                }
                do {
                    modules = try await model.installModule(bundle: bundle)
                    notice = L10n.tr("Installed. Approve what it asks for to use it.")
                } catch {
                    notice = error.localizedDescription
                }
            }
        }
    }

    private func load() async {
        do {
            modules = try await model.modules()
            loaded = true
        } catch {
            self.error = error.localizedDescription
        }
    }
}

/// One module in the list: its name, what it is, and its state.
private struct ModuleRow: View {
    let module: HostModule
    let open: () -> Void

    var body: some View {
        Button(action: open) {
            HStack(spacing: 12) {
                Image(systemName: module.symbol ?? "puzzlepiece.extension")
                    .font(.system(size: 14))
                    .frame(width: 32, height: 32)
                    .background(Palette.bubbleAgent, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .foregroundStyle(Palette.text)
                VStack(alignment: .leading, spacing: 2) {
                    Text(module.name).foregroundStyle(Palette.text)
                    Text(subtitle).font(.footnote).foregroundStyle(Palette.secondary).lineLimit(2)
                }
                Spacer(minLength: 8)
                Image(systemName: "chevron.right").font(.caption.weight(.semibold)).foregroundStyle(Palette.tertiary)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var subtitle: String {
        var parts: [String] = []
        if let version = module.version { parts.append("v\(version)") }
        parts.append(L10n.tr(module.stateText))
        parts.append(L10n.tr(module.approvedText))
        return parts.joined(separator: " · ")
    }
}

/// One module: what it asks for, what was approved, its switch, its screens, and taking it off.
private struct ModuleSheet: View {
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var module: HostModule
    @State private var screen: ModuleScreen?
    @State private var error: String?
    @State private var busy = false
    @State private var confirming = false
    @State private var shared: URL?
    let apply: ([HostModule]) -> Void

    init(module: HostModule, apply: @escaping ([HostModule]) -> Void) {
        _module = State(initialValue: module)
        self.apply = apply
    }

    private var permissions: ModulePermissions? { module.permissions }
    private var nothingApproved: Bool {
        module.approved.tools.isEmpty && module.approved.events.isEmpty && module.approved.files == "none"
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Module") {
                IconButton("Close", systemImage: "xmark") { dismiss() }
            }
            CardForm {
                if let broken = module.broken {
                    CardSection("It can’t run") {
                        Text(broken).font(.footnote).foregroundStyle(Palette.danger)
                    }
                }
                CardSection(module.description.isEmpty ? nil : module.description) {
                    HStack(spacing: 10) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(module.version.map { "v\($0)" } ?? module.id).foregroundStyle(Palette.text)
                            Text("\(L10n.tr(module.stateText)) · \(L10n.tr(module.approvedText))")
                                .font(.caption).foregroundStyle(Palette.secondary)
                        }
                        Spacer(minLength: 8)
                        Toggle("On", isOn: Binding(get: { module.enabled }, set: { on in
                            Task { await change { try await model.setModule(module.id, enabled: on) } }
                        }))
                        .toggleStyle(.codync)
                        .labelsHidden()
                        .disabled(busy || module.broken != nil)
                    }
                    if let stopped = module.stopped {
                        Text(L10n.tr("Last stopped: {why}").replacingOccurrences(of: "{why}", with: L10n.tr(stopped)))
                            .font(.caption).foregroundStyle(Palette.secondary)
                    }
                }
                if let permissions {
                    CardSection(nothingApproved
                        ? "Approve what this module may do. A bot can only call what you approve."
                        : L10n.tr("Approved: {what}.").replacingOccurrences(of: "{what}", with: L10n.tr(module.approvedText))) {
                        PermissionLine("Tools", asked: permissions.tools, allowed: module.approved.tools)
                        PermissionLine("Events", asked: permissions.events, allowed: module.approved.events)
                        PermissionLine("Hosts", asked: permissions.network, allowed: module.approved.network)
                        HStack(spacing: 8) {
                            Text("Files").font(.footnote).foregroundStyle(Palette.text)
                            Text(permissions.files == "none" ? L10n.tr("None")
                                : "\(module.approved.files == "none" ? "·" : "✓") \(permissions.files)")
                                .font(.caption).foregroundStyle(Palette.secondary)
                        }
                        HStack(spacing: 8) {
                            Button(nothingApproved ? "Approve everything it asks for" : "Approve all of it") {
                                Task { await change { try await model.approveModule(module.id) } }
                            }
                            .buttonStyle(PrimaryButtonStyle())
                            .disabled(busy)
                            if !nothingApproved {
                                Button("Approve nothing") {
                                    Task {
                                        await change {
                                            try await model.approveModule(module.id, permissions: ModulePermissions())
                                        }
                                    }
                                }
                                .buttonStyle(.secondary)
                                .disabled(busy)
                            }
                        }
                    }
                }
                if !module.views.isEmpty {
                    CardSection("Screens") {
                        HStack(spacing: 8) {
                            ForEach(module.views) { view in
                                Button(view.title) {
                                    Task { await load(view.id) }
                                }
                                .buttonStyle(.secondary)
                                .disabled(!module.enabled)
                            }
                        }
                        if let screen {
                            ForEach(screen.sections, id: \.title) { section in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(section.title).font(.caption).foregroundStyle(Palette.secondary)
                                    ForEach(section.rows, id: \.label) { row in
                                        HStack(spacing: 8) {
                                            Text(row.label).font(.caption).foregroundStyle(Palette.secondary)
                                            Text(row.value).font(.caption).foregroundStyle(Palette.text)
                                        }
                                    }
                                }
                            }
                        }
                        if let error {
                            Text(error).font(.caption).foregroundStyle(Palette.danger)
                        }
                    }
                }
                CardSection(module.folder) {
                    HStack(spacing: 8) {
                        Button("Save to a file") { Task { await export() } }
                            .buttonStyle(.secondary)
                        Button("Remove") { confirming = true }
                            .buttonStyle(.secondary)
                            .disabled(busy)
                    }
                    Text("Removing takes its files off this computer; the bots keep working without its tools.")
                        .font(.caption).foregroundStyle(Palette.tertiary)
                }
            }
        }
        .background(Palette.background)
        .codyncDialog(L10n.tr("Remove {name}?").replacingOccurrences(of: "{name}", with: module.name),
            isPresented: $confirming,
            message: "Its files leave this computer and the bots lose its tools. What the bots already did stays.") {
            [DialogAction("Remove", destructive: true) {
                Task {
                    do {
                        apply(try await model.uninstallModule(module.id))
                        dismiss()
                    } catch {
                        self.error = error.localizedDescription
                    }
                }
            }]
        }
        .codyncSheet(item: Binding(get: { shared.map { SharedModuleFile(url: $0) } }, set: { if $0 == nil { shared = nil } })) { file in
            FileExportSheet(url: file.url) { shared = nil }
        }
    }

    /// One change that answers with the whole list: the sheet follows its module.
    private func change(_ work: @escaping () async throws -> [HostModule]) async {
        busy = true
        do {
            let list = try await work()
            apply(list)
            if let fresh = list.first(where: { $0.id == module.id }) { module = fresh }
        } catch {
            self.error = error.localizedDescription
        }
        busy = false
    }

    private func load(_ view: String) async {
        error = nil
        do {
            screen = try await model.moduleView(module.id, view: view)
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func export() async {
        do {
            let saved = try await model.exportModule(module.id)
            let data = try JSONEncoder().encode(saved.bundle)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent(saved.name)
            try data.write(to: url)
            shared = url
        } catch {
            self.error = error.localizedDescription
        }
    }
}

/// A file waiting for the share sheet, by its URL.
private struct SharedModuleFile: Identifiable {
    var id: String { url.absoluteString }
    let url: URL
}

/// One permission, with a check on each item the owner approved.
private struct PermissionLine: View {
    let label: String
    let asked: [String]
    let allowed: [String]

    init(_ label: String, asked: [String], allowed: [String]) {
        self.label = label
        self.asked = asked
        self.allowed = allowed
    }

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Text(L10n.tr(label)).font(.footnote).foregroundStyle(Palette.text)
            Text(asked.isEmpty ? L10n.tr("Nothing")
                : asked.map { "\(allowed.contains($0) ? "✓" : "·") \($0)" }.joined(separator: "  "))
                .font(.caption).foregroundStyle(Palette.secondary)
        }
    }
}
