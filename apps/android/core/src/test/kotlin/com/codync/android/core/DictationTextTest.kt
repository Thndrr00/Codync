package com.codync.android.core

import org.junit.Assert.assertEquals
import org.junit.Test

class DictationTextTest {
    @Test fun `words go after the box with one space`() {
        assertEquals("hello", DictationText.join("  ", "hello"))
        assertEquals("Fix it", DictationText.join("Fix it", ""))
        assertEquals("Fix it now", DictationText.join("Fix it", "now"))
        assertEquals("Fix it\nnow", DictationText.join("Fix it\n", "now"))
    }

    @Test fun `parts cover the recording and respect the 90 s limit`() {
        val part = DictationSplit.PART_BYTES
        val loud = { _: Int, _: Int -> 1.0 }
        assertEquals(listOf(0 until part / 2), DictationSplit.ranges(part / 2, loud))
        assertEquals(listOf(0 until part, part until part * 2), DictationSplit.ranges(part * 2, loud))
        assertEquals(emptyList<IntRange>(), DictationSplit.ranges(0, loud))
        val ranges = DictationSplit.ranges(part * 2 + 1, loud)
        assertEquals(2, ranges.size)
        assertEquals(0, ranges.first().start)
        assertEquals(part * 2 + 1, ranges.last().endInclusive + 1)
        assertEquals(true, ranges.all { it.endInclusive - it.start + 1 <= part })
        assertEquals(Pcm.RATE * 2 * 600, DictationSplit.MAX_BYTES)
    }

    @Test fun `a long recording breaks at the quietest nearby point`() {
        val part = DictationSplit.PART_BYTES
        val second = Pcm.RATE * 2
        val count = part * 2 - second
        val quiet = part - second
        val ranges = DictationSplit.ranges(count) { start, _ -> if (start == quiet) 0.0 else 1.0 }
        assertEquals(2, ranges.size)
        assertEquals(0 until quiet, ranges[0])
        assertEquals(quiet until count, ranges[1])
    }

    @Test fun `loudness reads pcm amplitude`() {
        assertEquals(0.0, DictationSplit.level(ByteArray(20), 0, 20), 0.0)
        val pcm = ByteArray(20)
        for (i in 0 until 10) {
            pcm[i * 2] = (-1).toByte()
            pcm[i * 2 + 1] = 0x7f.toByte()
        }
        assertEquals(true, DictationSplit.level(pcm, 0, 20) > 0.6)
    }
}
