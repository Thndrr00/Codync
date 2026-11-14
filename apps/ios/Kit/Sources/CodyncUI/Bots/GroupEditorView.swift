import CodyncKit
import PhotosUI
import SwiftUI

/// Change a group chat's name, title, picture, what it's for, its lead, who's in it, and whether it is archived.
/// Bots are picked like recipients: chips on top, a search, and the bots not in it yet.
public struct GroupEditorView: View {
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    let groupId: String
    @State private var name: String
    @State private var title: String
    @State private var about: String
    @State private var members: [String]
    @State private var chiefBotId: String?
    @State private var archived: Bool
    @State private var keepMemories: Bool
    @State private var memory: MemoryStore?
    @State private var photo: String?
    @State private var photoItem: PhotosPickerItem?
    @State private var pickingPhoto = false
    @State private var photoError: String?
    @State private var query = ""
    @State private var adding = false
    @State private var saving = false
    @State private var error: String?
    @State private var confirmDelete: Bot?

    public init(group: Bot) {
        groupId = group.id
        _name = State(initialValue: group.name)
        _title = State(initialValue: group.title ?? "")
        _about = State(initialValue: group.description)
        _members = State(initialValue: group.members)
        _chiefBotId = State(initialValue: group.chiefBotId)
        _archived = State(initialValue: group.archived)
        _keepMemories = State(initialValue: group.keepMemories)
        _photo = State(initialValue: group.avatarPhoto)
    }

    private var picked: [Bot] { members.compactMap { model.bots[$0] } }
    /// Bots that can still join, filtered by the search.
    private var candidates: [Bot] {
        let q = query.trimmingCharacters(in: .whitespaces)
        return model.roster.filter { !$0.isGroup && !members.contains($0.id) && (q.isEmpty || $0.name.localizedCaseInsensitiveContains(q)) }
    }
    private var canSave: Bool { !members.isEmpty && !saving }

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Group chat") {
                if saving {
                    Spinner()
                } else {
                    IconButton("Save", systemImage: "checkmark") { save() }
                        .disabled(!canSave)
                }
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    VStack(spacing: 10) {
                        GroupAvatar(members: picked, size: 84, photo: photo)
                            .animation(Motion.layout, value: members)
                        TextField(defaultName, text: $name)
                            .textFieldStyle(.plain)
                            .font(.title2.weight(.semibold))
                            .foregroundStyle(Palette.text)
                            .multilineTextAlignment(.center)
                            .padding(.top, 6)
                        TextField(L10n.tr("A short line under the name"), text: $title)
                            .textFieldStyle(.plain)
                            .font(.subheadline)
                            .foregroundStyle(Palette.secondary)
                            .multilineTextAlignment(.center)
                        TextField("What this group works on", text: $about, axis: .vertical)
                            .textFieldStyle(.plain)
                            .lineLimit(1...4)
                            .font(.subheadline)
                            .foregroundStyle(Palette.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    VStack(spacing: 14) {
                        ValueRow("Picture", detail: "Shown in the roster instead of the members' faces.") {
                            HStack(spacing: 10) {
                                Button(L10n.tr(photo == nil ? "Choose Picture" : "Change Picture")) { pickingPhoto = true }
                                    .buttonStyle(SecondaryButtonStyle())
                                if photo != nil {
                                    Button(L10n.tr("Remove Picture")) { photo = nil }
                                        .buttonStyle(SecondaryButtonStyle())
                                }
                            }
                        }
                        if let photoError {
                            Text(L10n.tr(photoError)).font(.caption).foregroundStyle(Palette.danger)
                        }
                        ValueRow("Archived", detail: "Archived groups leave the open roster; their conversation stays.") {
                            Toggle("", isOn: $archived).toggleStyle(.codync)
                                .accessibilityLabel(L10n.tr("Archived"))
                        }
                        ValueRow("Keep memories in this channel", detail: "What its members learn here stays in this channel; their other chats don't get it.") {
                            Toggle("", isOn: $keepMemories).toggleStyle(.codync)
                                .accessibilityLabel(L10n.tr("Keep memories in this channel"))
                        }
                    }
                    VStack(alignment: .leading, spacing: 0) {
                        Text(L10n.tr("Memory"))
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(Palette.secondary)
                            .padding(.bottom, 4)
                        Text(L10n.tr("Every member reads these here. To change one, ask a member to remember or forget it."))
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                        if let entries = memory?.view?.channel?.entries, !entries.isEmpty {
                            ForEach(entries) { entry in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(entry.text).foregroundStyle(Palette.text)
                                    Text("\(author(entry)) · \(Date(milliseconds: entry.updatedAt).formatted(.relative(presentation: .named)))")
                                        .font(.caption)
                                        .foregroundStyle(Palette.secondary)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.vertical, 6)
                            }
                        } else {
                            Text(L10n.tr("Nothing yet. What the channel learns shows up here."))
                                .font(.caption)
                                .foregroundStyle(Palette.tertiary)
                                .padding(.vertical, 6)
                        }
                    }
                    DocumentsView(botId: groupId)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(L10n.tr("Members"))
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(Palette.secondary)
                            .padding(.bottom, 4)
                        ForEach(picked) { bot in
                            memberRow(bot) {
                                IconButton("Remove \(bot.name)", systemImage: "xmark") { toggle(bot.id) }
                            }
                            .transition(.opacity)
                        }
                        if adding {
                            TextField("Search bots", text: $query)
                                .fieldBox(fill: Palette.surface)
                                .onSubmit { if let first = candidates.first { toggle(first.id) } }
                                .padding(.vertical, 6)
                            ForEach(candidates) { bot in
                                Button { toggle(bot.id) } label: {
                                    memberRow(bot, dimmed: true) {
                                        Image(systemName: "plus.circle").font(.system(size: 18)).foregroundStyle(Palette.secondary)
                                    }
                                }
                                .buttonStyle(PressScale())
                                .accessibilityLabel("Add \(bot.name)")
                            }
                        } else if !candidates.isEmpty {
                            Button { withAnimation(Motion.layout) { adding = true } } label: {
                                HStack(spacing: 14) {
                                    Image(systemName: "plus").font(.system(size: 16)).foregroundStyle(Palette.secondary).frame(width: 32)
                                    Text(L10n.tr("Add Member")).foregroundStyle(Palette.secondary)
                                    Spacer(minLength: 0)
                                }
                                .frame(minHeight: 48)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(PressScale())
                        }
                        Text(L10n.tr("Everyone answers in turn unless you @mention someone."))
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                            .padding(.top, 10)
                    }
                    .animation(Motion.layout, value: members)
                    if !picked.isEmpty {
                        ValueRow("Chief of Staff", detail: "Coordinates delegated assignments and automatically intervenes if a member stalls or fails.") {
                            ChoicePicker(
                                selection: Binding(get: { chiefBotId ?? "" }, set: { chiefBotId = $0.isEmpty ? nil : $0 }),
                                options: [(id: "", label: L10n.tr("None (auto-detect Chief)"))]
                                    + picked.map { (id: $0.id, label: $0.name) }
                            )
                        }
                    }
                    if let error {
                        Text(L10n.tr(error)).font(.footnote).foregroundStyle(Palette.danger).transition(.opacity)
                    }
                    if let group = model.bots[groupId] {
                        HistoryActions(bot: group)
                        if archived {
                            Button(L10n.tr("Restore group chat")) { archived = false; save() }
                                .buttonStyle(SecondaryButtonStyle())
                        }
                        Button(L10n.tr("Delete group chat"), role: .destructive) { confirmDelete = group }
                            .foregroundStyle(Palette.danger)
                            .buttonStyle(PressScale())
                    }
                }
                .padding(.horizontal, 20)
                .padding(.top, 8)
                .padding(.bottom, 24)
            }
            .scrollDismissesKeyboard(.interactively)
        }
        .background(Palette.background)
        .photosPicker(isPresented: $pickingPhoto, selection: $photoItem, matching: .images)
        .onChange(of: photoItem) { _, item in
            guard let item else { return }
            Task {
                if let data = try? await item.loadTransferable(type: Data.self),
                   let jpeg = ImageData.channelPicture(data), jpeg.count <= 300 * 1024 {
                    photo = "data:image/jpeg;base64," + jpeg.base64EncodedString()
                    photoError = nil
                } else {
                    photoError = L10n.tr("The picture could not be read.")
                }
                photoItem = nil
            }
        }
        .task(id: "\(groupId)-\(model.bots[groupId]?.memoryRev ?? 0)") {
            let store = memory ?? MemoryStore(botId: groupId) { try await model.ready() }
            memory = store
            await store.load()
        }
        .deleteBotConfirmation($confirmDelete) { dismiss() }
    }

    /// Who wrote a channel memory, as the memory screen says it.
    private func author(_ entry: MemoryEntry) -> String {
        switch entry.author {
        case "user": return L10n.tr("You wrote it")
        case "bot": return "\(model.bots[entry.authorBot ?? ""]?.name ?? "A bot") remembered it"
        case "dream": return L10n.tr("Dreaming")
        default: return L10n.tr("Moved from the old memory")
        }
    }

    /// "Alice, Bob" when no name is typed.
    private var defaultName: String {
        let names = picked.map(\.name)
        return names.isEmpty ? "Name" : names.joined(separator: ", ")
    }

    private func toggle(_ id: String) {
        withAnimation(Motion.layout) {
            if members.contains(id) { members.removeAll { $0 == id } } else { members.append(id) }
        }
        query = ""
    }

    /// A member or candidate: avatar, name, and a trailing control (Grok Bot's member list).
    private func memberRow(_ bot: Bot, dimmed: Bool = false, @ViewBuilder trailing: () -> some View) -> some View {
        HStack(spacing: 14) {
            CharacterAvatar(bot: bot, size: 32, animated: false)
            Text(bot.name).foregroundStyle(dimmed ? Palette.secondary : Palette.text).lineLimit(1)
            Spacer(minLength: 0)
            trailing()
        }
        .frame(minHeight: 48)
        .contentShape(Rectangle())
    }

    private func save() {
        saving = true
        error = nil
        let typed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        let finalName = typed.isEmpty ? defaultName : typed
        let about = about.trimmingCharacters(in: .whitespacesAndNewlines)
        let typedTitle = title.trimmingCharacters(in: .whitespacesAndNewlines)
        Task {
            do {
                var draft = GroupDraft(name: finalName, description: about, members: members)
                draft.id = groupId
                draft.pinned = model.bots[groupId]?.pinned
                draft.title = typedTitle.isEmpty ? nil : typedTitle
                draft.archived = archived
                draft.keepMemories = keepMemories
                draft.avatarPhoto = photo
                draft.chiefBotId = chiefBotId
                try await model.updateGroup(draft)
                dismiss()
            } catch {
                withAnimation(Motion.fade) { self.error = error.localizedDescription }
            }
            saving = false
        }
    }
}
