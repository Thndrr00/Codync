package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

// Modules (`host/src/modules/`): a signed folder the host runs in its own process. Its manifest names the tools a
// bot may call, the events it may hear, the hosts it may reach and how much a call may cost.

/** What a module asks for, and what the owner approved of it. */
@Serializable
data class ModulePermissions(
    val tools: List<String> = emptyList(),
    val events: List<String> = emptyList(),
    val network: List<String> = emptyList(),
    /** `none`, `read` or `write`: how much of a bot's folder it may touch. */
    val files: String = "none",
)

/** What one call may cost; the host holds every module to it. */
@Serializable
data class ModuleLimits(
    val callsPerMinute: Int = 60,
    val timeMs: Long = 20_000,
    val memoryMb: Int = 256,
)

@Serializable
data class ModuleTool(val name: String, val description: String = "")

@Serializable
data class ModuleViewRef(val id: String, val title: String)

/** A module installed on this computer. */
@Serializable
data class HostModule(
    val id: String,
    val name: String,
    val version: String? = null,
    val description: String = "",
    val symbol: String? = null,
    val contract: Int? = null,
    val enabled: Boolean = false,
    /** Why it can't run, when its folder no longer verifies. */
    val broken: String? = null,
    /** `running`, `stopped`, `disabled` or `broken`. */
    val state: String = "stopped",
    /** Why its process stopped, when it did. */
    val stopped: String? = null,
    val permissions: ModulePermissions? = null,
    val approved: ModulePermissions = ModulePermissions(),
    val limits: ModuleLimits = ModuleLimits(),
    val views: List<ModuleViewRef> = emptyList(),
    val tools: List<ModuleTool> = emptyList(),
    val installedAt: Long = 0,
    val folder: String = "",
) {
    val stateText: String get() = when {
        broken != null -> "Broken"
        !enabled -> "Off"
        state == "running" -> "Running"
        else -> "Ready"
    }

    /** What the owner approved, in one line. */
    val approvedText: String get() {
        val parts = buildList {
            if (approved.tools.isNotEmpty()) add("${approved.tools.size} tools")
            if (approved.events.isNotEmpty()) add("${approved.events.size} events")
            if (approved.files != "none") add("${approved.files} files")
        }
        return parts.ifEmpty { listOf("Nothing approved yet") }.joinToString(" · ")
    }
}

/** A module's screen, as rows the apps draw with their own components. */
@Serializable
data class ModuleScreen(val title: String = "", val sections: List<Section> = emptyList()) {
    @Serializable
    data class Section(val title: String = "", val rows: List<Row> = emptyList())

    @Serializable
    data class Row(val label: String = "", val value: String = "")
}

@Serializable
data class ModulesReply(val modules: List<HostModule> = emptyList())

@Serializable
data class ModuleViewReply(val view: ModuleScreen = ModuleScreen())

@Serializable
data class ModuleExport(val name: String, val bundle: JsonElement)

/** The bodies of the module methods (`host/src/modules/api.rs`). */
object ModuleCalls {
    fun none(): JsonObject = buildJsonObject {}

    fun set(id: String, enabled: Boolean): JsonObject = buildJsonObject { put("id", id); put("enabled", enabled) }

    fun approve(id: String, permissions: ModulePermissions?): JsonObject = buildJsonObject {
        put("id", id)
        if (permissions == null) put("permissions", kotlinx.serialization.json.JsonNull) else put("permissions", WireJson.encodeToJsonElement(permissions))
    }

    fun install(folder: String? = null, bundle: JsonElement? = null): JsonObject = buildJsonObject {
        folder?.let { put("folder", it) }
        bundle?.let { put("bundle", it) }
    }

    fun id(id: String): JsonObject = buildJsonObject { put("id", id) }

    fun view(id: String, view: String): JsonObject = buildJsonObject { put("id", id); put("view", view) }
}
