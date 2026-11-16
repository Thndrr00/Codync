package com.codync.android

/** How an attached file is getting to the computer. */
sealed interface UploadStatus {
    data object Uploading : UploadStatus
    data object Uploaded : UploadStatus
    data class Failed(val reason: String) : UploadStatus
    data object Cancelled : UploadStatus
}

/** One attached file on its way to the computer: what it has, and why it stopped (iOS `FileUpload`). */
data class FileUpload(
    /** The draft file's path, which names it in the message box. */
    val path: String,
    /** The id the computer knows the upload by (`cancelUpload`, then `send`). */
    val uploadId: String,
    val botId: String,
    val status: UploadStatus,
    val sent: Long,
    val total: Long,
) {
    val percent: Int get() = if (total <= 0) 0 else ((sent * 100) / total).toInt().coerceIn(0, 100)
    val isUploading: Boolean get() = status == UploadStatus.Uploading
    /** A file that stopped without reaching the computer: it stays in the box until retried or removed. */
    val hasFailed: Boolean get() = status is UploadStatus.Failed || status == UploadStatus.Cancelled
    val reason: String get() = (status as? UploadStatus.Failed)?.reason ?: "The upload was cancelled."
}
