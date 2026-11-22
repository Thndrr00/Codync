package com.codync.android

import com.codync.android.core.ThreadInfo
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun thread(id: String, at: Long) = ThreadInfo(id = id, archived = true, createdAt = at, updatedAt = at)

class ThreadPagingTest {
    @Test fun aFullPageOffersAnotherAndKeepsSavedRows() {
        val first = (0 until ThreadPaging.pageSize).map { thread("t$it", (100 - it).toLong()) }
        val (rows, more) = ThreadPaging.merge(first, listOf(thread("older", 1)))
        assertFalse(more)
        assertEquals(ThreadPaging.pageSize + 1, rows.size)
        assertEquals("older", rows.last().id)
        assertEquals("older", ThreadPaging.cursor(rows))
    }

    @Test fun aHostThatIgnoresTheCursorStopsPaging() {
        val first = (0 until ThreadPaging.pageSize).map { thread("t$it", (100 - it).toLong()) }
        assertFalse(ThreadPaging.merge(first, first).second)
        assertTrue(ThreadPaging.merge(emptyList(), first).second)
    }

    @Test fun aPartialPageStaysNewestFirstAndHasNoCursorWhenEmpty() {
        val (rows, more) = ThreadPaging.merge(listOf(thread("new", 200)), listOf(thread("old", 100), thread("older", 50)))
        assertFalse(more)
        assertEquals(listOf("new", "old", "older"), rows.map { it.id })
        assertNull(ThreadPaging.cursor(emptyList()))
    }
}
