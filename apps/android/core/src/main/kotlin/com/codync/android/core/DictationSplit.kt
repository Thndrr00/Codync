package com.codync.android.core

/**
 * How a long dictation recording is cut for transcription (iOS `DictationSplit`, OpenBot nightly
 * t3-b2cd3129). One part holds at most 90 seconds and ends at the quietest half-second near that
 * limit, so a word is not cut in two. A recording runs for at most ten minutes. Pure: the split and
 * a piece's loudness can be tested without a microphone or a provider.
 */
object DictationSplit {
    private const val BYTES_PER_SECOND = Pcm.RATE * 2
    /** A dictation records for up to ten minutes; the pill stops at this point. */
    const val MAX_BYTES = BYTES_PER_SECOND * 60 * 10
    /** The longest part sent to a speech-to-text provider. */
    const val PART_BYTES = BYTES_PER_SECOND * 90
    /** How far back from the limit a quiet point is looked for. */
    private const val LOOK_BACK_BYTES = BYTES_PER_SECOND * 10
    /** A quiet point is half a second of the quietest audio near the limit. */
    private const val QUIET_BYTES = BYTES_PER_SECOND / 2
    /** A tail under half a second stays with the part before it. */
    private const val MIN_TAIL_BYTES = BYTES_PER_SECOND / 2
    /** Never make a part shorter than this just to find a quiet point. */
    private const val MIN_PART_BYTES = BYTES_PER_SECOND * 45
    /** Only a genuinely quiet window moves a cut back from the limit; steady speech waits. */
    private const val QUIET_LEVEL = 0.15

    /** Byte ranges (start until end) covering [count] bytes, each at most [PART_BYTES]. */
    fun ranges(count: Int, loudness: (start: Int, end: Int) -> Double): List<IntRange> {
        if (count <= 0) return emptyList()
        val parts = mutableListOf<IntRange>()
        var start = 0
        while (count - start > PART_BYTES) {
            val limit = start + PART_BYTES
            val window = maxOf(start + MIN_PART_BYTES, limit - LOOK_BACK_BYTES)
            var cut = limit
            var quietest = Double.POSITIVE_INFINITY
            var probe = window
            while (probe < limit) {
                val end = minOf(probe + QUIET_BYTES, limit)
                val level = loudness(probe, end)
                if (level < QUIET_LEVEL && level < quietest) {
                    quietest = level
                    cut = probe
                }
                probe = end
            }
            parts.add(start until cut)
            start = cut
        }
        if (start < count) parts.add(start until count)
        // A very short tail would make the provider invent a word for what is barely a tap.
        if (parts.size > 1 && count - parts.last().start < MIN_TAIL_BYTES) {
            parts[parts.size - 2] = parts[parts.size - 2].start until count
            parts.removeAt(parts.lastIndex)
        }
        return parts
    }

    /** Mean amplitude of 16 kHz mono PCM16 bytes from [start] until [end], 0…1. */
    fun level(pcm: ByteArray, start: Int, end: Int): Double {
        var i = start
        var sum = 0.0
        var samples = 0
        while (i + 1 < end) {
            val word = (pcm[i].toInt() and 0xff) or (pcm[i + 1].toInt() shl 8)
            val value = word.toShort().toDouble() / 32768.0
            sum += value * value
            samples++
            i += 2
        }
        return if (samples == 0) 0.0 else kotlin.math.sqrt(sum / samples)
    }
}
