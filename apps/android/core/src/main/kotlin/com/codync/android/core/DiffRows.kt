package com.codync.android.core

/**
 * A unified diff split into rows for display (idea 26). Viewing never applies the patch. Line
 * numbers appear only where a hunk's `@@ -a,b +c,d @@` ranges let them be counted; a diff
 * that does not parse (or is huge) returns null so the caller shows the original text.
 */
object DiffRows {
    sealed interface Row {
        /** The line exactly as written, prefix included. */
        val raw: String

        data class Header(override val raw: String) : Row
        data class Hunk(override val raw: String) : Row
        data class Note(override val raw: String) : Row
        data class Context(override val raw: String, val oldNumber: Int?, val newNumber: Int?) : Row
        data class Added(override val raw: String, val newNumber: Int?) : Row
        data class Removed(override val raw: String, val oldNumber: Int?) : Row
    }

    const val MAX_LINES = 2_000

    private val hunkPattern = Regex("""^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@""")
    private val headerPrefixes = listOf("diff ", "index ", "--- ", "+++ ", "new file", "deleted file", "old mode", "new mode",
        "similarity index", "dissimilarity index", "rename ", "copy ", "Binary files", "GIT binary patch")

    /** Whether a fenced block is, or looks like, a unified diff. */
    fun looksLikeDiff(language: String, text: String): Boolean {
        val tag = language.trim().lowercase()
        if (tag == "diff" || tag == "patch") return true
        return tag.isEmpty() && text.lineSequence().firstOrNull { it.isNotBlank() }?.let { it.startsWith("diff --git ") || hunkPattern.containsMatchIn(it) } == true
    }

    fun parse(text: String): List<Row>? {
        val lines = text.split("\n").map { it.removeSuffix("\r") }
        if (lines.size > MAX_LINES) return null
        val rows = mutableListOf<Row>()
        var oldLeft = 0; var newLeft = 0
        var old = 0; var new = 0
        var recognised = false
        for (line in lines) {
            val inHunk = oldLeft > 0 || newLeft > 0
            val hunk = if (inHunk) null else hunkPattern.find(line)
            when {
                hunk != null -> {
                    val (a, b, c, d) = hunk.destructured
                    old = a.toInt(); oldLeft = b.ifEmpty { "1" }.toInt()
                    new = c.toInt(); newLeft = d.ifEmpty { "1" }.toInt()
                    rows += Row.Hunk(line); recognised = true
                }
                line.startsWith("\\") -> rows += Row.Note(line)
                !inHunk && headerPrefixes.any(line::startsWith) -> { rows += Row.Header(line); recognised = true }
                line.startsWith("+") -> {
                    rows += Row.Added(line, if (newLeft > 0) new else null)
                    if (newLeft > 0) { new++; newLeft-- }
                }
                line.startsWith("-") -> {
                    rows += Row.Removed(line, if (oldLeft > 0) old else null)
                    if (oldLeft > 0) { old++; oldLeft-- }
                }
                line.startsWith(" ") || (line.isEmpty() && inHunk) -> {
                    rows += Row.Context(line, if (oldLeft > 0) old else null, if (newLeft > 0) new else null)
                    if (oldLeft > 0) { old++; oldLeft-- }
                    if (newLeft > 0) { new++; newLeft-- }
                }
                line.isEmpty() -> rows += Row.Note(line)
                else -> return null
            }
        }
        // A trailing newline leaves one empty line that is not part of the patch.
        if (rows.lastOrNull() == Row.Note("")) rows.removeAt(rows.lastIndex)
        return if (recognised) rows else null
    }
}
