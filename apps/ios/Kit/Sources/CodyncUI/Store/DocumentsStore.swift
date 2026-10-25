import CodyncKit
import Foundation
import Observation

struct LibraryDocument: Decodable, Identifiable {
    let id: String
    let botId: String
    let parentId: String?
    let title: String
    let revision: Int64
    let createdAt: Int64
    let updatedAt: Int64
    var body: String?
}

struct DocumentDraft: Codable, Equatable {
    var id: String?
    var clientNonce: String?
    var parentId: String?
    var title: String
    var body: String
    var ifRevision: Int64
}

struct DocumentReview: Decodable, Identifiable {
    let id: String
    let botId: String
    let title: String?
    let status: String
    let draft: DocumentDraft?
    let document: LibraryDocument?
    let createdAt: Int64
    let expiresAt: Int64
}

struct DocumentActivity: Decodable, Identifiable {
    let id: Int64
    let action: String
    let actorKind: String
    let actorId: String
    let result: String
    let createdAt: Int64
}

/// The host owns persistence, revisions and review decisions. Drafts survive failed writes.
@MainActor @Observable
final class DocumentsStore {
    let botId: String
    private let client: () async throws -> HostClient
    private let backupKey: String
    var documents: [LibraryDocument] = []
    var reviews: [DocumentReview] = []
    var activity: [DocumentActivity] = []
    var query = ""
    var draft: DocumentDraft?
    var conflict: LibraryDocument?
    var selectedReview: DocumentReview?
    var loading = false
    var saving = false
    var opening = false
    var reviewing = false
    var error: String?
    var editorError: String?
    var reviewError: String?
    var saveStatus = ""
    var next: String?
    var dirty = false
    private var loadRequest = 0
    private var editorRequest = 0
    private var reviewRequest = 0
    private var openedID: String?
    private var autosave: Task<Void, Never>?

    init(botId: String, accountContext: String, computerId: String, client: @escaping () async throws -> HostClient) {
        self.botId = botId
        self.client = client
        backupKey = "documents.draft.\(SharedStore.Context.digest(accountContext + ":" + computerId + ":" + botId))"
        if let data = UserDefaults.standard.data(forKey: backupKey) {
            do {
                draft = try JSONDecoder().decode(DocumentDraft.self, from: data)
                dirty = true
                saveStatus = "Recovered unsaved draft"
            } catch { editorError = "Couldn't recover the saved draft: \(error.localizedDescription)" }
        }
    }

    private struct Listing: Decodable { let items: [LibraryDocument]; let next: String? }
    private struct Reviews: Decodable { let items: [DocumentReview] }
    private struct Activities: Decodable { let items: [DocumentActivity] }
    private struct DocumentResult: Decodable { let document: LibraryDocument }
    private struct ReviewResult: Decodable { let review: DocumentReview }
    private struct WriteResult: Decodable { let status: String; let document: LibraryDocument? }
    private struct ID: Encodable { let id: String }
    private struct Bot: Encodable { let botId: String }

    func load(more: Bool = false) async {
        loadRequest += 1
        let asked = loadRequest
        let search = query
        loading = true
        error = nil
        defer { if asked == loadRequest { loading = false } }
        struct Body: Encodable { let botId: String; let query: String; let afterId: String?; let limit: Int }
        do {
            let response: Listing = try await client().call("listDocuments", Body(botId: botId, query: search, afterId: more ? next : nil, limit: 100))
            guard asked == loadRequest else { return }
            documents = more ? documents + response.items : response.items
            next = response.next
            if !more { await loadSupporting() }
        } catch is CancellationError {} catch {
            if asked == loadRequest { self.error = error.localizedDescription }
        }
    }

    func loadSupporting() async {
        do {
            let response: Reviews = try await client().call("listDocumentReviews", Bot(botId: botId))
            reviews = response.items
            let history: Activities = try await client().call("listComputerActivity", Bot(botId: botId))
            activity = history.items
        } catch is CancellationError {} catch { self.error = error.localizedDescription }
    }

    func create(parentId: String? = nil) {
        autosave?.cancel()
        editorRequest += 1
        draft = DocumentDraft(clientNonce: UUID().uuidString, parentId: parentId, title: "", body: "", ifRevision: 0)
        conflict = nil
        editorError = nil
        opening = false
        dirty = false
        saveStatus = "New document"
    }

    func open(_ id: String) async {
        autosave?.cancel()
        editorRequest += 1
        let asked = editorRequest
        openedID = id
        draft = nil
        conflict = nil
        opening = true
        editorError = nil
        defer { if asked == editorRequest { opening = false } }
        do {
            let response: DocumentResult = try await client().call("getDocument", ID(id: id))
            guard asked == editorRequest else { return }
            accept(response.document)
        } catch is CancellationError {} catch {
            if asked == editorRequest { editorError = error.localizedDescription }
        }
    }

    private func accept(_ document: LibraryDocument) {
        draft = DocumentDraft(id: document.id, parentId: document.parentId, title: document.title,
                              body: document.body ?? "", ifRevision: document.revision)
        conflict = nil
        editorError = nil
        dirty = false
        saveStatus = "Saved"
        UserDefaults.standard.removeObject(forKey: backupKey)
    }

    func edit(title: String? = nil, body: String? = nil) {
        guard var value = draft else { return }
        if let title { value.title = title }
        if let body { value.body = body }
        draft = value
        dirty = true
        saveStatus = "Unsaved changes"
        backup()
        // A failure or conflict requires an explicit retry; typing never dismisses it.
        guard conflict == nil, editorError == nil else { return }
        scheduleSave()
    }

    private func scheduleSave() {
        autosave?.cancel()
        autosave = Task {
            do { try await Task.sleep(for: .milliseconds(800)) }
            catch { return }
            autosave = nil
            await save()
        }
    }

    func save(overwrite: Bool = false) async {
        guard !saving, dirty, var sent = draft, !sent.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        if overwrite, let conflict { sent.ifRevision = conflict.revision }
        else if conflict != nil { return }
        let asked = editorRequest
        saving = true
        editorError = nil
        saveStatus = "Saving…"
        defer {
            saving = false
            if dirty, conflict == nil, editorError == nil,
               let draft, !draft.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                scheduleSave()
            }
        }
        struct Body: Encodable {
            let botId: String; let id: String?; let clientNonce: String?; let parentId: String?
            let title: String; let body: String; let ifRevision: Int64
        }
        do {
            let response: WriteResult = try await client().call("saveDocument", Body(botId: botId, id: sent.id, clientNonce: sent.id == nil ? sent.clientNonce : nil,
                parentId: sent.id == nil ? sent.parentId : nil, title: sent.title, body: sent.body, ifRevision: sent.ifRevision))
            guard asked == editorRequest, let document = response.document else { return }
            if response.status == "conflict" {
                conflict = document
                saveStatus = "Changed on another device"
            } else {
                // A retried creation may return its first committed snapshot. Preserve
                // newer local text and continue as a revision-checked update to that ID.
                let unchanged = draft?.title == document.title && draft?.body == (document.body ?? "")
                if unchanged { accept(document) }
                else {
                    draft?.id = document.id
                    draft?.clientNonce = nil
                    draft?.parentId = document.parentId
                    draft?.ifRevision = document.revision
                    backup()
                    saveStatus = "Unsaved changes"
                    scheduleSave()
                }
                await load()
            }
        } catch is CancellationError { saveStatus = "Unsaved changes" }
        catch { editorError = error.localizedDescription; saveStatus = "Not saved" }
    }

    func reloadConflict() {
        autosave?.cancel()
        if let conflict { accept(conflict) }
    }

    func retryEditor() async {
        if draft == nil, let openedID { await open(openedID) }
        else { await save() }
    }

    func discardDraft() {
        guard !saving else { return }
        autosave?.cancel()
        editorRequest += 1
        draft = nil
        conflict = nil
        editorError = nil
        dirty = false
        UserDefaults.standard.removeObject(forKey: backupKey)
    }

    private func backup() {
        guard let draft else { return }
        do { UserDefaults.standard.set(try JSONEncoder().encode(draft), forKey: backupKey) }
        catch { editorError = "Couldn't back up this draft: \(error.localizedDescription)" }
    }

    func remove() async -> Bool {
        guard !saving, let draft, let id = draft.id else { return false }
        saving = true
        defer { saving = false }
        struct Body: Encodable { let id: String; let ifRevision: Int64 }
        do {
            let response: WriteResult = try await client().call("deleteDocument", Body(id: id, ifRevision: draft.ifRevision))
            if response.status == "conflict" { conflict = response.document; return false }
            self.draft = nil
            dirty = false
            await load()
            return true
        } catch { editorError = error.localizedDescription; return false }
    }

    func move(parentId: String?) async {
        guard !saving, !dirty, let draft, let id = draft.id else { return }
        saving = true
        editorError = nil
        defer { saving = false }
        struct Body: Encodable {
            let id: String; let parentId: String?; let ifRevision: Int64
            enum CodingKeys: String, CodingKey { case id, parentId, ifRevision }
            func encode(to encoder: any Encoder) throws {
                var values = encoder.container(keyedBy: CodingKeys.self)
                try values.encode(id, forKey: .id)
                try values.encode(parentId, forKey: .parentId)
                try values.encode(ifRevision, forKey: .ifRevision)
            }
        }
        do {
            let response: WriteResult = try await client().call("moveDocument", Body(id: id, parentId: parentId, ifRevision: draft.ifRevision))
            if response.status == "conflict" { conflict = response.document }
            else if let document = response.document { accept(document); await load() }
        } catch { editorError = error.localizedDescription }
    }

    func inspectReview(_ id: String) async {
        reviewRequest += 1
        let asked = reviewRequest
        selectedReview = nil
        reviewError = nil
        reviewing = true
        defer { if asked == reviewRequest { reviewing = false } }
        do {
            let response: ReviewResult = try await client().call("getDocumentReview", ID(id: id))
            if asked == reviewRequest { selectedReview = response.review }
        } catch { if asked == reviewRequest { reviewError = error.localizedDescription } }
    }

    func resolveReview(approve: Bool) async {
        guard !reviewing, let selectedReview, selectedReview.status == "pending" else { return }
        reviewing = true
        reviewError = nil
        defer { reviewing = false }
        struct Body: Encodable { let id: String; let decision: String }
        do {
            let response: ReviewResult = try await client().call("resolveAction", Body(id: selectedReview.id, decision: approve ? "approve" : "decline"))
            self.selectedReview = response.review
            await load()
        } catch { reviewError = error.localizedDescription }
    }

    func exportActivity() async -> String? {
        struct CSV: Decodable { let csv: String }
        do {
            let response: CSV = try await client().call("exportComputerActivity", Bot(botId: botId))
            return response.csv
        } catch { self.error = error.localizedDescription; return nil }
    }
}
