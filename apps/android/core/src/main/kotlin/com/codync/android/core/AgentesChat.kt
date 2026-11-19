package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** Behaviors brought from agentes.chat. The host owns them; this file only calls and decodes. */

@Serializable
data class PresenceStatus(val text: String, val until: Long? = null)

@Serializable
data class SetupStep(val id: String, val done: Boolean, val label: String)

@Serializable
data class SetupChecklist(val steps: List<SetupStep>, val done: Int, val total: Int, val complete: Boolean)

@Serializable
data class StretchCard(
    val type: String,
    val agent: String,
    val reason: String,
    val action: String,
    val bodyCode: String,
)

@Serializable
data class ApprovalOption(val id: String, val label: String, val kind: String)

@Serializable
data class ApprovalRequest(
    val type: String,
    val requestId: String,
    val agent: String,
    val reason: String,
    val options: List<ApprovalOption>,
    val status: String,
    val selected: String? = null,
)

@Serializable
data class VaultEntry(val name: String, val source: String, val masked: String)

private val chatJson = Json { ignoreUnknownKeys = true }

suspend fun EncryptedChannel.presence(botId: String): PresenceStatus? =
    chatJson.decodeFromJsonElement(call("botPresence", buildJsonObject { put("botId", botId) })
        .jsonObject["status"] ?: return null)

suspend fun EncryptedChannel.setPresence(botId: String, status: String, until: Long? = null): PresenceStatus? =
    chatJson.decodeFromJsonElement(call("botPresence", buildJsonObject {
        put("botId", botId); put("status", status); until?.let { put("until", it) }
    }).jsonObject["status"] ?: return null)

suspend fun EncryptedChannel.setup(botId: String): SetupChecklist =
    chatJson.decodeFromJsonElement(call("botSetup", buildJsonObject { put("botId", botId) }))

suspend fun EncryptedChannel.stretchCard(agent: String, reason: String): StretchCard =
    chatJson.decodeFromJsonElement(call("stretchCard", buildJsonObject {
        put("agent", agent); put("reason", reason)
    }))

suspend fun EncryptedChannel.approvalCard(agent: String, reason: String, requestId: String): ApprovalRequest =
    chatJson.decodeFromJsonElement(call("agentApproval", buildJsonObject {
        put("agent", agent); put("reason", reason); put("requestId", requestId)
    }))

suspend fun EncryptedChannel.vaultMask(name: String, secret: String, source: String = "keychain"): VaultEntry =
    chatJson.decodeFromJsonElement(call("vaultMask", buildJsonObject {
        put("name", name); put("secret", secret); put("source", source)
    }))
