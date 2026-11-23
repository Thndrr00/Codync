package com.codync.android.core

/**
 * A header name is an HTTP token: letters, digits and a small set of separators,
 * never a space, colon, slash or bracket (OpenBot nightly triage-13174). The form,
 * the host and an agent all refuse the same names.
 */
object HeaderNames {
    const val MESSAGE =
        "Header names cannot contain spaces or symbols such as : ( ) /. Use a name such as Authorization, and put the key or token in Value."

    private val TOKEN = Regex("^[!#$%&'*+.^_`|~0-9A-Za-z-]+$")

    fun valid(name: String): Boolean = name.isNotEmpty() && name.length <= 64 && TOKEN.matches(name)

    /** The first unusable key in a "Name: value" text box, or null when every row is fine. */
    fun invalidKey(text: String): String? = text.lines().firstNotNullOfOrNull { line ->
        val at = line.indexOf(':')
        if (at < 0) null else line.substring(0, at).trim().takeIf { it.isNotEmpty() && !valid(it) }
    }
}
