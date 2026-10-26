package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.jsonObject

@Serializable
data class BotDraft(
    val id: String? = null,
    val name: String = "",
    val description: String = "",
    val avatarColor: String = AVATAR_COLORS.random(),
    val avatarShape: String = AVATAR_SHAPES.random(),
    val backend: String = "claude",
    val command: String? = null,
    val cwd: String = "",
    val permission: String = "auto",
    val model: String? = null,
    val modelOptions: Map<String, String> = emptyMap(),
    val fastMode: Boolean? = null,
    val pinned: Boolean = false,
    val hidden: Boolean = false,
    val notify: Boolean? = null,
    val connectors: List<String>? = null,
    val skills: List<String> = emptyList(),
    val computer: Boolean = false,
    val account: String? = null,
    val computerPermissions: ComputerPermissions = ComputerPermissions(),
) {
    val isValid: Boolean get() = id == null || name.isNotBlank()
    fun toggleNotifications(): BotDraft = copy(notify = !(notify ?: true))
    fun body(): JsonObject {
        val normalized = copy(command = command.takeIf { backend == "custom" }, account = account.takeIf { backend == "antigravity" })
        // updateBot merges a patch: null clears model, command or the connector choice, but an unset
        // switch must stay out (the host's booleans don't take null).
        return JsonObject(WireJson.encodeToJsonElement(normalized).jsonObject.filterNot { (key, value) -> value == JsonNull && key in SWITCHES })
    }
    fun template(): String = JsonObject(body().filterKeys { it !in setOf("id", "pinned", "hidden", "account") }).toString()

    companion object {
        /** A new bot starts with a random face, like iOS. */
        val AVATAR_COLORS = listOf("black", "brown", "red", "orange", "yellow", "green", "cyan", "blue", "violet", "magenta", "gray")
        private val SWITCHES = setOf("fastMode", "notify")
        val AVATAR_SHAPES = listOf("blob", "pebble", "squircle", "tablet", "wedge", "hex", "cloud", "teardrop")

        fun from(bot: Bot): BotDraft = BotDraft(bot.id, bot.name, bot.description, bot.avatarColor, bot.avatarShape,
            bot.backend, bot.command, if (bot.managedWorkspace) "" else bot.cwd, bot.permission, bot.model,
            bot.modelOptions, bot.fastMode, bot.pinned, bot.hidden, bot.notify, bot.connectors, bot.skills, bot.computer, bot.account, bot.computerPermissions)
    }
}
