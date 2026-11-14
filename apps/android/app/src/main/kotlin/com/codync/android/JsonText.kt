package com.codync.android

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonPrimitive

/** A text field, or null when it is missing, JSON null or not text (`Entry.data` helpers). */
internal fun JsonObject.text(key: String): String? = this[key]?.jsonPrimitive?.contentOrNull
