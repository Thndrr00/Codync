package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** Memory several bots share: a project's, or the Fleet one, which stands over the memory every bot used to share. */
@Serializable data class Brain(
    val id: String,
    val name: String,
    /** The Fleet brain: its layer is the old shared-memory folder. */
    val shared: Boolean = false,
    /** The bots working in it; only on the brains list. */
    val members: List<String> = emptyList(),
)

@Serializable data class BrainsReply(val brains: List<Brain> = emptyList())

/** The answer to an import: the brains, and the files the checks turned away. */
@Serializable data class BrainImportReply(val brains: List<Brain> = emptyList(), val skipped: List<Skipped> = emptyList()) {
    @Serializable data class Skipped(val name: String, val why: String = "")
}

/** A brain as one JSON file: its name and its files, so another computer can hold the same memory. */
@Serializable data class BrainBundle(
    val format: String,
    val version: Int,
    val brain: Named,
    val files: List<File>,
) {
    @Serializable data class Named(val name: String)
    @Serializable data class File(val name: String, val content: String)
}

/** The request bodies of the brain methods (CodyncKit's `HostClient` brain calls). */
object BrainCalls {
    fun list(): JsonObject = buildJsonObject {}

    fun name(id: String?, name: String): JsonObject = buildJsonObject {
        id?.let { put("id", it) }
        put("name", name)
    }

    fun join(botId: String, brainId: String): JsonObject = buildJsonObject {
        put("botId", botId); put("brainId", brainId)
    }

    fun bot(botId: String): JsonObject = buildJsonObject { put("botId", botId) }

    fun id(id: String): JsonObject = buildJsonObject { put("id", id) }

    fun import(bundle: JsonElement, name: String?): JsonObject = buildJsonObject {
        put("bundle", bundle)
        name?.let { put("name", it) }
    }

    /** `createBrain`, or `renameBrain` with an id. */
    fun saveMethod(id: String?): String = if (id != null) "renameBrain" else "createBrain"
}
