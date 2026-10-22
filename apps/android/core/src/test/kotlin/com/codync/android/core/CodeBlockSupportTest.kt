package com.codync.android.core

import com.codync.android.core.CodeHighlight.Kind
import com.codync.android.core.CodeHighlight.Span
import com.codync.android.core.DiffRows.Row
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CodeBlockSupportTest {
    @Test fun `highlight marks keywords comments strings and numbers`() {
        val code = "val x = 42 // n\nval s = \"a\""
        assertEquals(listOf(Span(0, 3, Kind.Keyword), Span(8, 10, Kind.Number), Span(11, 15, Kind.Comment),
            Span(16, 19, Kind.Keyword), Span(24, 27, Kind.Literal)), CodeHighlight.spans(code, "kotlin"))
    }

    @Test fun `unknown languages and huge blocks stay plain`() {
        assertTrue(CodeHighlight.spans("val x = 1", "brainfuck").isEmpty())
        assertTrue(CodeHighlight.spans("val x = 1\n".repeat(CodeHighlight.MAX_CHARS), "kotlin").isEmpty())
    }

    @Test fun `an unclosed string ends at its line`() {
        assertEquals(listOf(Span(0, 4, Kind.Literal), Span(5, 8, Kind.Keyword)), CodeHighlight.spans("'abc\nfun", "kotlin"))
    }

    @Test fun `diff rows number lines only inside the hunk ranges`() {
        val rows = DiffRows.parse("diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -3,2 +3,2 @@\n ctx\n-old\n+new\n+extra")!!
        assertEquals(Row.Header("--- a/f"), rows[1])
        assertEquals(listOf(Row.Context(" ctx", 3, 3), Row.Removed("-old", 4), Row.Added("+new", 4), Row.Added("+extra", null)), rows.drop(4))
    }

    @Test fun `text that is not a diff stays text`() {
        assertNull(DiffRows.parse("just some words"))
        assertNull(DiffRows.parse("+added but no hunk header\nplain"))
        assertTrue(DiffRows.looksLikeDiff("diff", ""))
        assertTrue(!DiffRows.looksLikeDiff("kotlin", "diff --git a b"))
    }

    @Test fun `binary diffs keep their header without invented lines`() {
        assertEquals(listOf(Row.Header("diff --git a/x.png b/x.png"), Row.Header("Binary files a/x.png and b/x.png differ")),
            DiffRows.parse("diff --git a/x.png b/x.png\nBinary files a/x.png and b/x.png differ"))
    }

    @Test fun `wrapping a selection edits one range and keeps it selected`() {
        val edit = ComposerEdits.wrapInCode("run ls now", 4, 6)!!
        assertEquals(ComposerEdits.Edit(4, 6, "`ls`", 5, 7), edit)
    }

    @Test fun `a reversed selection wraps the same as a forward one`() {
        assertEquals(ComposerEdits.wrapInCode("run ls now", 4, 6), ComposerEdits.wrapInCode("run ls now", 6, 4))
    }

    @Test fun `several lines become a fenced block on their own lines`() {
        val edit = ComposerEdits.wrapInCode("a\nb", 0, 3)!!
        assertEquals("```\na\nb\n```", edit.replacement)
        assertEquals(4, edit.selectionStart); assertEquals(7, edit.selectionEnd)
    }

    @Test fun `an empty selection inserts a pair and a selection with a fence is refused`() {
        assertEquals(ComposerEdits.Edit(2, 2, "``", 3, 3), ComposerEdits.wrapInCode("hi", 2, 2))
        assertNull(ComposerEdits.wrapInCode("```x```", 0, 7))
    }

    @Test fun `a block starts on a new line after text`() {
        assertEquals("\n```\n\n```", ComposerEdits.insertCodeBlock("hi", 2, 2)!!.replacement)
    }

    @Test fun `quote prefixes each selected line`() {
        assertEquals("> a\n> b", ComposerEdits.quote("a\nb\nc", 0, 3).replacement)
    }
}
