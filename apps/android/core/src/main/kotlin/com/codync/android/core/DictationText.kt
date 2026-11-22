package com.codync.android.core

/** Dictation's arithmetic (iOS `Dictation`'s statics): where its words go in the box, and seconds. */
object DictationText {
    private const val BYTES_PER_SECOND = Pcm.RATE * 2
    /** Under half a second is a tap of the mic, not speech (and speech-to-text models invent words for silence). */
    const val MIN_BYTES = BYTES_PER_SECOND / 2
    /** Transcription requests in flight at once, under the channel's and the providers' limits. */
    const val IN_FLIGHT = 3

    /** [said] after what's already in the box, with one space between. */
    fun join(before: String, said: String): String = when {
        before.isBlank() -> said
        said.isEmpty() -> before
        before.last().isWhitespace() -> before + said
        else -> "$before $said"
    }

    /** Seconds of audio in [bytes] of PCM, for this month's minutes. */
    fun seconds(bytes: Int): Long = (bytes / BYTES_PER_SECOND).toLong()
}
