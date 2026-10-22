package com.codync.android.core

/**
 * What the system's Share sheet may hand to Codync. The content only ever becomes a draft in a bot the user picks;
 * it is never sent on its own. The attachment flow still checks every file's size and storage.
 */
object ShareIntake {
    const val MAX_FILES = 10
    const val MAX_TEXT = 20_000
    /** Room for the merged text in the 64 KiB draft, beside the composer's own text and its file names. */
    const val MAX_DRAFT_BYTES = 48 * 1024

    /** MIME types the manifest accepts: plain text, images and PDFs. */
    fun accepts(mime: String?): Boolean {
        val type = mime?.substringBefore(';')?.trim()?.lowercase() ?: return false
        return type == "text/plain" || type == "application/pdf" || type.startsWith("image/")
    }

    /** One text for the draft: the subject (a page title) over the shared text, trimmed, never past [MAX_TEXT]. */
    fun draftText(subject: String?, text: String?): String {
        val parts = listOfNotNull(subject?.trim()?.takeIf { it.isNotEmpty() && it != text?.trim() }, text?.trim()?.takeIf { it.isNotEmpty() })
        return parts.joinToString("\n\n").take(MAX_TEXT)
    }

    /** The draft text of a bot's composer once a share arrives: appended after what the user already typed. */
    fun merge(existing: String, shared: String): String {
        val merged = when {
            shared.isBlank() -> existing
            existing.isBlank() -> shared
            else -> existing.trimEnd() + "\n\n" + shared
        }
        return utf8Prefix(merged, MAX_DRAFT_BYTES)
    }

    /** The longest prefix of [text] that fits [maxBytes] in UTF-8, never cutting a character in half. */
    internal fun utf8Prefix(text: String, maxBytes: Int): String {
        val bytes = text.toByteArray(Charsets.UTF_8)
        if (bytes.size <= maxBytes) return text
        var cut = maxBytes
        // A continuation byte (10xxxxxx) starts inside a character: back up to its first byte.
        while (cut > 0 && (bytes[cut].toInt() and 0xC0) == 0x80) cut--
        return String(bytes, 0, cut, Charsets.UTF_8)
    }
}
