package com.codync.android.core

/**
 * Composer shortcuts as pure edits (idea 27). Each returns one replacement of a range, never the
 * whole draft, so the field keeps its undo history and the caller only moves the selection. No
 * shortcut sends anything.
 */
object ComposerEdits {
    /** Replace `text[start, end)` with [replacement], then select `[selectionStart, selectionEnd]` of the new text. */
    data class Edit(val start: Int, val end: Int, val replacement: String, val selectionStart: Int, val selectionEnd: Int)

    private const val FENCE = "```"

    /**
     * Wraps the selection in code: a single line becomes inline code, several lines a fenced
     * block. With nothing selected, inserts an empty pair and puts the cursor between. A selection
     * that already holds a fence is left alone (null), since it would end the new block early.
     */
    fun wrapInCode(text: String, selStart: Int, selEnd: Int): Edit? {
        val (start, end) = ordered(text, selStart, selEnd)
        val selected = text.substring(start, end)
        if (FENCE in selected) return null
        return when {
            selected.isEmpty() -> inlineCode("", start, end)
            '\n' in selected -> fenced(text, selected, start, end)
            else -> inlineCode(selected, start, end)
        }
    }

    /** Inserts a fenced block on its own lines, around the selection when there is one. */
    fun insertCodeBlock(text: String, selStart: Int, selEnd: Int): Edit? {
        val (start, end) = ordered(text, selStart, selEnd)
        val selected = text.substring(start, end)
        return if (FENCE in selected) null else fenced(text, selected, start, end)
    }

    /** Prefixes every line of the selection (or the cursor's line) with a quote marker. */
    fun quote(text: String, selStart: Int, selEnd: Int): Edit {
        val (start, end) = ordered(text, selStart, selEnd)
        val lineStart = text.lastIndexOf('\n', start - 1) + 1
        val lineEnd = text.indexOf('\n', end).let { if (it < 0) text.length else it }
        val quoted = text.substring(lineStart, lineEnd).split("\n").joinToString("\n") { "> $it" }
        return Edit(lineStart, lineEnd, quoted, lineStart, lineStart + quoted.length)
    }

    private fun inlineCode(selected: String, start: Int, end: Int): Edit {
        val replacement = "`$selected`"
        return if (selected.isEmpty()) Edit(start, end, replacement, start + 1, start + 1)
            else Edit(start, end, replacement, start + 1, start + 1 + selected.length)
    }

    private fun fenced(text: String, selected: String, start: Int, end: Int): Edit {
        val before = if (start > 0 && text[start - 1] != '\n') "\n" else ""
        val after = if (end < text.length && text[end] != '\n') "\n" else ""
        val open = "$before$FENCE\n"
        val replacement = "$open$selected\n$FENCE$after"
        val bodyStart = start + open.length
        return if (selected.isEmpty()) Edit(start, end, replacement, bodyStart, bodyStart)
            else Edit(start, end, replacement, bodyStart, bodyStart + selected.length)
    }

    /** The selection as `start <= end`, clamped to the text; a reversed drag is the same range. */
    private fun ordered(text: String, a: Int, b: Int): Pair<Int, Int> =
        minOf(a, b).coerceIn(0, text.length) to maxOf(a, b).coerceIn(0, text.length)
}
