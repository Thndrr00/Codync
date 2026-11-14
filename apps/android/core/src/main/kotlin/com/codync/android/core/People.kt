package com.codync.android.core

import kotlinx.serialization.Serializable

/**
 * Another person in a chat: one of your devices, named and coloured by the computer (`presence`),
 * so two people on one computer are told apart.
 */
@Serializable
data class Person(val key: String = "", val name: String = "", val color: String = "", val typing: Boolean = false) {
    /** Up to two letters of the name, for its small face. */
    val initials: String get() = name.split(' ').filter { it.isNotBlank() }.take(2)
        .joinToString("") { word -> word.take(1).uppercase() }
}
