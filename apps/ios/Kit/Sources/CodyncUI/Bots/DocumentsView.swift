import CodyncKit
import SwiftUI

/// Bot-owned documents, human review and metadata-only computer history.
struct DocumentsView: View {
    let botId: String
    @Environment(BotStore.self) private var host
    @State private var library: DocumentsStore?
    @State private var editor = false
    @State private var review = false
    @State private var search = ""
    @State private var copied = false
    @State private var conversation: ThreadTarget?

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            if let library {
                searchField
                if let error = library.error {
                    Text(error).foregroundStyle(Palette.danger).textSelection(.enabled)
                    Button("Retry") { Task { await library.load() } }.buttonStyle(.secondary)
                }
                documents(library)
                reviews(library)
                activity(library)
            } else { Text("Loading documents…").foregroundStyle(Palette.secondary) }
        }
        .task(id: botId) {
            let model = DocumentsStore(botId: botId, accountContext: host.storage.id, computerId: host.computer.id) { try await host.ready() }
            library = model
            await model.load()
            while !Task.isCancelled {
                do { try await Task.sleep(for: .seconds(5)) } catch { return }
                if !model.loading && !model.saving {
                    if model.documents.count > 100 { await model.loadSupporting() }
                    else { await model.load() }
                }
            }
        }
        .task(id: search) {
            do { try await Task.sleep(for: .milliseconds(300)) } catch { return }
            library?.query = search
            await library?.load()
        }
        .codyncSheet(isPresented: $editor) {
            if let library { DocumentEditor(library: library) }
        }
        .codyncSheet(isPresented: $review) {
            if let library { DocumentReviewView(library: library) }
        }
        .codyncSheet(item: $conversation) { target in
            RepliesView(botId: botId, rootId: target.id) {
                withAnimation(Motion.layout) { conversation = nil }
            }
        }
    }

    private var searchField: some View {
        HStack(spacing: 10) {
            Image(systemName: "magnifyingglass").foregroundStyle(Palette.secondary)
            TextField("Search documents", text: $search).textFieldStyle(.plain)
                .accessibilityLabel("Search documents")
        }
        .padding(12).background(Palette.surface, in: RoundedRectangle(cornerRadius: 12))
    }

    private func documents(_ library: DocumentsStore) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Documents").font(.headline)
                Spacer()
                IconButton("Refresh documents", systemImage: "arrow.clockwise") {
                    Task { await library.load() }
                }.disabled(library.loading)
                IconButton("New document", systemImage: "plus") {
                    library.create()
                    withAnimation(Motion.layout) { editor = true }
                }.disabled(library.dirty || library.saving)
            }
            if library.draft != nil {
                Button(library.dirty ? "Resume unsaved document" : "Reopen document") {
                    withAnimation(Motion.layout) { editor = true }
                }.buttonStyle(.secondary)
            }
            if library.loading { Spinner() }
            if library.documents.isEmpty && !library.loading {
                Text(search.isEmpty ? "No documents yet. Create a Markdown document to get started." : "No documents match your search.")
                    .foregroundStyle(Palette.secondary)
            }
            ForEach(library.documents) { document in
                HStack(spacing: 8) {
                    Button {
                        withAnimation(Motion.layout) { editor = true }
                        Task { await library.open(document.id) }
                    } label: {
                        HStack(spacing: 10) {
                            Image(systemName: document.parentId == nil ? "doc.text" : "doc.on.doc")
                            VStack(alignment: .leading, spacing: 3) {
                                Text(document.title).foregroundStyle(Palette.text)
                                Text("Revision \(document.revision) · \(date(document.updatedAt))")
                                    .font(.caption).foregroundStyle(Palette.secondary)
                            }
                            Spacer(minLength: 0)
                            Image(systemName: "chevron.right").font(.caption).foregroundStyle(Palette.secondary)
                        }.frame(minHeight: 44).contentShape(Rectangle())
                    }.buttonStyle(.plain)
                    IconButton("Discuss document: \(document.title)", systemImage: "bubble.left.and.bubble.right") {
                        withAnimation(Motion.layout) { conversation = ThreadTarget(id: "doc:\(document.id)") }
                    }
                }.disabled(library.dirty || library.saving || library.opening)
            }
            if library.next != nil {
                Button("Load more") { Task { await library.load(more: true) } }
                    .buttonStyle(.secondary).disabled(library.loading)
            }
        }
    }

    private func reviews(_ library: DocumentsStore) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Human review").font(.headline)
            if library.reviews.isEmpty { Text("No review requests.").foregroundStyle(Palette.secondary) }
            ForEach(library.reviews) { request in
                Button {
                    withAnimation(Motion.layout) { review = true }
                    Task { await library.inspectReview(request.id) }
                } label: {
                    HStack {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(request.title ?? "Document review").foregroundStyle(Palette.text)
                            Text(request.status.capitalized).font(.caption).foregroundStyle(Palette.secondary)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").foregroundStyle(Palette.secondary)
                    }.frame(minHeight: 44).contentShape(Rectangle())
                }.buttonStyle(.plain)
            }
        }
    }

    private func activity(_ library: DocumentsStore) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Computer activity").font(.headline)
                Spacer()
                IconButton(copied ? "CSV copied" : "Copy activity as CSV", systemImage: copied ? "checkmark" : "doc.on.doc") {
                    Task {
                        if let csv = await library.exportActivity() {
                            Pasteboard.copy(csv)
                            copied = true
                        }
                    }
                }
            }
            if copied { Text("Activity CSV copied.").font(.caption).foregroundStyle(Palette.secondary) }
            if library.activity.isEmpty { Text("No computer activity yet.").foregroundStyle(Palette.secondary) }
            ForEach(library.activity) { item in
                VStack(alignment: .leading, spacing: 3) {
                    Text(item.action).font(.subheadline)
                    Text("\(item.result.capitalized) · \(item.actorKind): \(item.actorId) · \(date(item.createdAt))")
                        .font(.caption).foregroundStyle(item.result == "failed" ? Palette.danger : Palette.secondary)
                }.textSelection(.enabled)
            }
        }
    }

    private func date(_ milliseconds: Int64) -> String {
        Date(timeIntervalSince1970: Double(milliseconds) / 1000).formatted(date: .abbreviated, time: .shortened)
    }
}

private struct DocumentEditor: View {
    @Bindable var library: DocumentsStore
    @State private var preview = false
    @State private var delete = false
    @State private var discard = false
    @Environment(\.dismissModal) private var dismiss

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Document") {
                if library.saving { Spinner() }
                IconButton(preview ? "Edit Markdown" : "Preview Markdown", systemImage: preview ? "pencil" : "eye") {
                    withAnimation(Motion.layout) { preview.toggle() }
                }
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    if library.opening { Spinner() }
                    if let error = library.editorError {
                        Text(error).foregroundStyle(Palette.danger).textSelection(.enabled)
                        Text("Your draft stays on this device. Retry when connected, or keep editing and save later.")
                            .font(.caption).foregroundStyle(Palette.secondary)
                        if library.dirty || library.draft == nil {
                            Button("Retry") { Task { await library.retryEditor() } }.buttonStyle(.secondary)
                        }
                    }
                    if let conflict = library.conflict {
                        Text("A newer revision is available. Your draft is preserved.").foregroundStyle(Palette.warning)
                        Text("Current revision: \(conflict.revision)").font(.caption)
                        DisclosureGroupContent(document: conflict)
                        Button("Reload current revision") { library.reloadConflict() }.buttonStyle(.secondary).disabled(library.saving)
                        Button("Overwrite with my draft") { Task { await library.save(overwrite: true) } }
                            .buttonStyle(.secondary).disabled(library.saving || !library.dirty)
                    }
                    if let draft = library.draft, !library.opening {
                        if preview {
                            Text(draft.title).font(.title2.weight(.semibold))
                            MarkdownText(draft.body)
                        } else {
                            TextField("Document title", text: Binding(get: { library.draft?.title ?? "" }, set: { library.edit(title: $0) }))
                                .font(.title2.weight(.semibold)).textFieldStyle(.plain)
                            TextField("Write Markdown…", text: Binding(get: { library.draft?.body ?? "" }, set: { library.edit(body: $0) }), axis: .vertical)
                                .lineLimit(14...60).font(.body.monospaced()).textFieldStyle(.plain).plainTextInput()
                        }
                        Text(library.saveStatus).font(.caption).foregroundStyle(Palette.secondary)
                            .accessibilityLabel("Save status").accessibilityValue(library.saveStatus)
                        Button("Save now") { Task { await library.save() } }.buttonStyle(.primary)
                            .disabled(library.saving || !library.dirty || draft.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        if library.dirty {
                            Button("Discard unsaved draft") { withAnimation(Motion.layout) { discard = true } }
                                .foregroundStyle(Palette.danger).disabled(library.saving)
                        }
                        if let id = draft.id {
                            ChoicePicker(selection: Binding(get: { library.draft?.parentId ?? "" }, set: { parent in
                                Task { await library.move(parentId: parent.isEmpty ? nil : parent) }
                            }), options: [("", "Library root")] + library.documents.filter { $0.id != id }.map { ($0.id, $0.title) })
                                .disabled(library.dirty || library.saving || library.conflict != nil)
                            Button("New child document") { library.create(parentId: id) }.buttonStyle(.secondary)
                                .disabled(library.dirty || library.saving)
                            Button("Delete document") { withAnimation(Motion.layout) { delete = true } }
                                .foregroundStyle(Palette.danger).disabled(library.saving || library.dirty)
                        }
                    }
                }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .background(Palette.background)
        .interactiveDismissDisabled(library.saving)
        .codyncDialog("Delete this document and its children?", isPresented: $delete) {
            [DialogAction("Delete", destructive: true) {
                Task { if await library.remove() { dismiss() } }
            }]
        }
        .codyncDialog("Discard unsaved changes?", isPresented: $discard) {
            [DialogAction("Discard", destructive: true) {
                library.discardDraft()
                dismiss()
            }]
        }
    }
}

private struct DisclosureGroupContent: View {
    let document: LibraryDocument
    @State private var expanded = false
    var body: some View {
        Disclosure(isExpanded: $expanded) {
            VStack(alignment: .leading, spacing: 8) {
                Text(document.title).font(.headline)
                Text(document.body ?? "").font(.footnote.monospaced()).textSelection(.enabled)
            }
        } label: { Text("Inspect current document") }
    }
}

private struct DocumentReviewView: View {
    @Bindable var library: DocumentsStore

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Review document")
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    if library.reviewing { Spinner() }
                    if let error = library.reviewError { Text(error).foregroundStyle(Palette.danger).textSelection(.enabled) }
                    if let review = library.selectedReview {
                        Text(review.status.capitalized).foregroundStyle(Palette.secondary)
                        Text("Expires \(Date(timeIntervalSince1970: Double(review.expiresAt) / 1000).formatted(date: .abbreviated, time: .shortened))")
                            .font(.caption).foregroundStyle(Palette.secondary)
                        if let draft = review.draft {
                            Text(draft.title).font(.title2.weight(.semibold))
                            Text("Base revision: \(draft.ifRevision)").font(.caption).foregroundStyle(Palette.secondary)
                            Text(draft.body).font(.body.monospaced()).textSelection(.enabled)
                            if review.status == "pending" {
                                Button("Approve this exact draft") { Task { await library.resolveReview(approve: true) } }
                                    .buttonStyle(.primary).disabled(library.reviewing)
                                Button("Decline") { Task { await library.resolveReview(approve: false) } }
                                    .buttonStyle(.secondary).disabled(library.reviewing)
                            }
                        }
                        if review.status == "conflict" {
                            Text("The document changed. This draft was not applied.").foregroundStyle(Palette.warning)
                        }
                    }
                }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
            }
        }.background(Palette.background)
    }
}
