package com.codync.android

import com.codync.android.core.HostFiles
import com.codync.android.core.OutgoingFile
import java.io.IOException
import java.util.UUID
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.update

// A message box's files reach the computer as they are attached, so the ring shows before Send
// names them, one failure or cancel never takes the others down, and a retry sends only that file.

/** Starts a file just attached; one already on its way or done is left alone. */
internal fun AppStore.startUpload(botId: String, file: OutgoingFile) {
    when (state.value.fileUploads[file.path]?.status) {
        UploadStatus.Uploading, UploadStatus.Uploaded -> return
        else -> beginUpload(botId, file)
    }
}

/** Stops a file's upload; the computer drops its partial chunks, and the file waits in the box. */
fun AppStore.cancelUpload(file: OutgoingFile) {
    val upload = state.value.fileUploads[file.path] ?: return
    if (upload.status != UploadStatus.Uploading) return
    uploadJobs.remove(file.path)?.cancel()
    mutableState.update { it.copy(fileUploads = it.fileUploads + (file.path to upload.copy(status = UploadStatus.Cancelled, sent = 0))) }
    contextScope.launch {
        runCatching { HostFiles(ready()).cancelUpload(upload.botId, upload.uploadId) }
    }
}

/** Starts a failed or cancelled file again, dropping the partial it left on the computer. */
fun AppStore.retryUpload(file: OutgoingFile) {
    val previous = state.value.fileUploads[file.path] ?: return
    if (previous.status == UploadStatus.Uploaded) return
    contextScope.launch {
        runCatching { HostFiles(ready()).cancelUpload(previous.botId, previous.uploadId) }
    }
    beginUpload(previous.botId, file)
}

/** The file left the box: its upload stops, and what the computer has of it goes too. */
fun AppStore.forgetUpload(file: OutgoingFile) {
    val upload = state.value.fileUploads[file.path] ?: return
    uploadJobs.remove(file.path)?.cancel()
    mutableState.update { it.copy(fileUploads = it.fileUploads - file.path) }
    if (upload.status != UploadStatus.Uploading) return
    contextScope.launch {
        runCatching { HostFiles(ready()).cancelUpload(upload.botId, upload.uploadId) }
    }
}

/** The message carrying them arrived: their rings are done with. */
internal fun AppStore.releaseUploads(files: List<OutgoingFile>) {
    if (files.isEmpty()) return
    mutableState.update { it.copy(fileUploads = it.fileUploads - files.map { file -> file.path }.toSet()) }
}

/**
 * Waits until every file is uploaded, then names their upload ids in order. A file that failed or
 * was cancelled starts again, so sending waits for it as the box says; its reason throws instead.
 */
internal suspend fun AppStore.uploadIds(botId: String, files: List<OutgoingFile>): List<String> {
    for (file in files) {
        if (state.value.fileUploads[file.path]?.status != UploadStatus.Uploaded) startUpload(botId, file)
    }
    while (true) {
        val uploads = files.map { state.value.fileUploads[it.path] }
        uploads.firstOrNull { it?.status is UploadStatus.Failed }?.let { throw IOException(it.reason) }
        uploads.firstOrNull { it?.status == UploadStatus.Cancelled }?.let { throw IOException(it.reason) }
        if (uploads.all { it?.status == UploadStatus.Uploaded }) return uploads.map { it!!.uploadId }
        delay(100)
    }
}

private fun AppStore.beginUpload(botId: String, file: OutgoingFile) {
    val uploadId = UUID.randomUUID().toString()
    mutableState.update {
        it.copy(fileUploads = it.fileUploads + (file.path to FileUpload(file.path, uploadId, botId, UploadStatus.Uploading, 0, file.size)))
    }
    uploadJobs.remove(file.path)?.cancel()
    uploadJobs[file.path] = contextScope.launch {
        try {
            HostFiles(ready()).upload(botId, uploadId, file) { sent -> noteUpload(file.path, uploadId, sent) }
            updateUpload(file.path, uploadId) { it.copy(status = UploadStatus.Uploaded, sent = file.size) }
        } catch (error: CancellationException) {
            // A cancel already said so; a chunk in flight may have put the partial back.
            contextScope.launch { runCatching { HostFiles(ready()).cancelUpload(botId, uploadId) } }
            throw error
        } catch (error: Exception) {
            updateUpload(file.path, uploadId) { it.copy(status = UploadStatus.Failed(error.message ?: "Upload failed")) }
        } finally {
            // A retry may already hold this file's entry; only its own run clears it.
            val current = state.value.fileUploads[file.path]
            if (current == null || current.uploadId == uploadId) uploadJobs.remove(file.path)
        }
    }
}

private fun AppStore.noteUpload(path: String, uploadId: String, sent: Long) {
    updateUpload(path, uploadId) { if (it.sent == sent) it else it.copy(sent = sent) }
}

/** Applies `change` to the file's current run; a run a retry replaced is left alone. */
private fun AppStore.updateUpload(path: String, uploadId: String, change: (FileUpload) -> FileUpload) {
    mutableState.update { state ->
        val upload = state.fileUploads[path]
        if (upload == null || upload.uploadId != uploadId) state
        else state.copy(fileUploads = state.fileUploads + (path to change(upload)))
    }
}
