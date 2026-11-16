import CodyncKit
import Foundation

/// One attached file on its way to the computer: how much of it the computer has, and why it stopped.
public struct FileUpload: Identifiable, Hashable, Sendable {
    public enum Status: Hashable, Sendable {
        case uploading
        case uploaded
        case failed(String)
        case cancelled
    }

    /// The attached file's own id (`OutgoingFile.id`).
    public var id: UUID
    /// The id the computer knows the upload by (`cancelUpload`, then `send`).
    public var uploadId: String
    public var botId: String
    public var status: Status
    /// Bytes the computer has, and the file's size.
    public var sent: Int
    public var total: Int

    public var percent: Int { total > 0 ? min(100, sent * 100 / total) : 0 }
    public var isUploading: Bool { status == .uploading }
    /// A file that stopped without reaching the computer: it stays in the box until retried or removed.
    public var hasFailed: Bool {
        if case .failed = status { return true }
        return status == .cancelled
    }
    public var reason: String {
        if case .failed(let reason) = status { return reason }
        return "The upload was cancelled."
    }
}

/// A file that never reached the computer; `errorDescription` is what the box shows.
public enum FileUploadError: LocalizedError {
    case failed(String)
    case cancelled

    public var errorDescription: String? {
        switch self {
        case .failed(let reason): reason
        case .cancelled: "The upload was cancelled."
        }
    }
}

extension BotStore {
    /// A file just attached: the computer starts receiving it now, so its ring shows before Send names it.
    func startUpload(_ file: OutgoingFile, for botId: String) {
        switch fileUploads[file.id]?.status {
        case .uploading, .uploaded: return
        default: beginUpload(file, for: botId)
        }
    }

    /// Stops a file's upload; the computer drops its partial chunks, and the file waits in the box.
    func cancelUpload(_ file: OutgoingFile) {
        guard let upload = fileUploads[file.id], upload.status == .uploading else { return }
        uploadTasks[file.id]?.cancel()
        uploadTasks[file.id] = nil
        fileUploads[file.id] = FileUpload(id: file.id, uploadId: upload.uploadId, botId: upload.botId,
                                           status: .cancelled, sent: 0, total: upload.total)
        settleUpload(file.id)
        Task { [weak self] in
            try? await self?.withLink(replay: false) { client in
                try await client.cancelUpload(botId: upload.botId, id: upload.uploadId)
            }
        }
    }

    /// Starts a failed or cancelled file again, dropping the partial it left on the computer.
    func retryUpload(_ file: OutgoingFile) {
        guard let previous = fileUploads[file.id] else { return }
        Task { [weak self] in
            try? await self?.withLink(replay: false) { client in
                try await client.cancelUpload(botId: previous.botId, id: previous.uploadId)
            }
        }
        beginUpload(file, for: previous.botId)
    }

    /// The file left the box: its upload stops, and what the computer has of it goes too.
    func forgetUpload(_ fileId: UUID) {
        guard let upload = fileUploads.removeValue(forKey: fileId) else { return }
        uploadTasks[fileId]?.cancel()
        uploadTasks[fileId] = nil
        settleUpload(fileId)
        guard upload.status == .uploading else { return }
        Task { [weak self] in
            try? await self?.withLink(replay: false) { client in
                try await client.cancelUpload(botId: upload.botId, id: upload.uploadId)
            }
        }
    }

    /// The message carrying them arrived: their rings are done with.
    func releaseUploads(_ files: [OutgoingFile]) {
        for file in files {
            fileUploads[file.id] = nil
            uploadTasks[file.id] = nil
        }
    }

    /// Waits until the file is uploaded, then names its upload id in a `send`. A file that failed or
    /// was cancelled starts again, so sending waits for it as the box shows; otherwise it throws.
    func uploadId(of file: OutgoingFile, for botId: String) async throws -> String {
        if let upload = fileUploads[file.id], upload.status == .uploaded { return upload.uploadId }
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<String, Error>) in
            uploadWaiters[file.id, default: []].append(continuation)
            if fileUploads[file.id] == nil || fileUploads[file.id]?.hasFailed == true {
                beginUpload(file, for: botId)
            }
        }
    }

    /// The ids of `files`, in order, uploaded under `botId`.
    func uploadIds(of files: [OutgoingFile], for botId: String) async throws -> [String] {
        var ids: [String] = []
        for file in files {
            try Task.checkCancellation()
            ids.append(try await uploadId(of: file, for: botId))
        }
        return ids
    }

    // MARK: internals

    private func beginUpload(_ file: OutgoingFile, for botId: String) {
        let uploadId = UUID().uuidString
        fileUploads[file.id] = FileUpload(id: file.id, uploadId: uploadId, botId: botId,
                                           status: .uploading, sent: 0, total: file.data.count)
        uploadTasks[file.id]?.cancel()
        uploadTasks[file.id] = Task { [weak self] in
            guard let self else { return }
            do {
                try await self.withLink(replay: false) { client in
                    try await client.upload(botId: botId, id: uploadId, name: file.name, data: file.data) { [weak self] sent in
                        Task { @MainActor in self?.noteUpload(uploadId, fileId: file.id, sent: sent) }
                    }
                }
                guard var upload = self.fileUploads[file.id], upload.uploadId == uploadId else { return }
                upload.status = .uploaded
                upload.sent = file.data.count
                self.fileUploads[file.id] = upload
                self.settleUpload(file.id)
            } catch is CancellationError {
                // `cancelUpload` already said so; a chunk still in flight may have put its partial back.
                try? await self.withLink(replay: false) { client in
                    try await client.cancelUpload(botId: botId, id: uploadId)
                }
            } catch {
                guard var upload = self.fileUploads[file.id], upload.uploadId == uploadId else { return }
                upload.status = .failed(error.localizedDescription)
                self.fileUploads[file.id] = upload
                self.settleUpload(file.id)
            }
            // A retry may already hold the map's entry for this file; only its own run clears it.
            if self.fileUploads[file.id]?.uploadId == uploadId {
                self.uploadTasks[file.id] = nil
            }
        }
    }

    private func noteUpload(_ uploadId: String, fileId: UUID, sent: Int) {
        guard var upload = fileUploads[fileId], upload.uploadId == uploadId, upload.status == .uploading,
              upload.sent != sent else { return }
        upload.sent = sent
        fileUploads[fileId] = upload
    }

    /// Answers the sends waiting for this file: its id once uploaded, why it stopped otherwise.
    private func settleUpload(_ fileId: UUID) {
        guard let upload = fileUploads[fileId] else {
            for waiter in uploadWaiters.removeValue(forKey: fileId) ?? [] { waiter.resume(throwing: FileUploadError.cancelled) }
            return
        }
        guard let waiters = uploadWaiters.removeValue(forKey: fileId) else { return }
        switch upload.status {
        case .uploaded:
            for waiter in waiters { waiter.resume(returning: upload.uploadId) }
        case .failed(let reason):
            for waiter in waiters { waiter.resume(throwing: FileUploadError.failed(reason)) }
        case .cancelled:
            for waiter in waiters { waiter.resume(throwing: FileUploadError.cancelled) }
        case .uploading:
            uploadWaiters[fileId] = waiters
        }
    }
}
