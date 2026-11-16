package com.codync.android

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put

/**
 * This computer's policy rules (`docs/features/policy-engine.md`): a named CEL expression with an effect
 * (`allow` or `deny`) and a message. The host compiles and decides; these calls only carry its answers.
 */
internal data class PolicyRule(
    val id: String,
    val name: String,
    val expression: String,
    val effect: String,
    val message: String,
    val enabled: Boolean,
)

/** A starting point the host offers; it is a rule only once it is saved. */
internal data class PolicyPreset(
    val id: String,
    val name: String,
    val about: String,
    val expression: String,
    val effect: String,
    val message: String,
)

/** What the policy list says about a proposed call (`previewPolicy`), and the variables it read. */
internal data class PolicyAnswer(
    val decision: String,
    val explanation: String,
    val ruleName: String?,
    val variables: List<Pair<String, String>>,
)

internal data class PolicyList(val rules: List<PolicyRule>, val presets: List<PolicyPreset>)

private fun JsonObject.text(key: String): String = (this[key] as? JsonPrimitive)?.contentOrNull ?: ""
private fun JsonObject.flag(key: String): Boolean = (this[key] as? JsonPrimitive)?.contentOrNull == "true"

private fun JsonObject.policyRule() = PolicyRule(
    id = text("id"),
    name = text("name"),
    expression = text("expression"),
    effect = text("effect").ifEmpty { "deny" },
    message = text("message"),
    enabled = flag("enabled"),
)

private fun JsonObject.policyPreset() = PolicyPreset(
    id = text("id"),
    name = text("name"),
    about = text("about"),
    expression = text("expression"),
    effect = text("effect").ifEmpty { "deny" },
    message = text("message"),
)

private fun policyBody(rule: PolicyRule): JsonObject = buildJsonObject {
    if (rule.id.isNotEmpty()) put("id", rule.id)
    put("name", rule.name.trim())
    put("expression", rule.expression.trim())
    put("effect", rule.effect)
    put("message", rule.message.trim())
    put("enabled", rule.enabled)
}

/** The list and its presets (`policies`). */
internal suspend fun AppStore.policyList(): PolicyList {
    val reply = hostCall("policies", buildJsonObject {}).jsonObject
    val rules = (reply["policies"] as? JsonArray)?.map { it.jsonObject.policyRule() } ?: emptyList()
    val presets = (reply["presets"] as? JsonArray)?.map { it.jsonObject.policyPreset() } ?: emptyList()
    return PolicyList(rules, presets)
}

/** Adds (`id` empty) or changes a rule; the host compiles it and refuses what it can't run, with why. */
internal suspend fun AppStore.savePolicy(rule: PolicyRule): PolicyRule {
    val body = buildJsonObject { put("policy", policyBody(rule)) }
    return hostCall("savePolicy", body).jsonObject["policy"]!!.jsonObject.policyRule()
}

internal suspend fun AppStore.setPolicyEnabled(id: String, enabled: Boolean) {
    val body = buildJsonObject {
        put("id", id)
        put("enabled", enabled)
    }
    hostCall("setPolicyEnabled", body)
}

internal suspend fun AppStore.removePolicy(id: String) {
    hostCall("removePolicy", body("id" to id))
}

/** What the policy list says about a proposed call, without acting. */
internal suspend fun AppStore.previewPolicy(tool: String, command: String, initiator: String): PolicyAnswer {
    val body = buildJsonObject {
        put("tool", tool)
        if (command.isNotBlank()) put("command", command)
        put("initiator", initiator)
    }
    val preview = hostCall("previewPolicy", body).jsonObject["preview"]!!.jsonObject
    val variables = buildList {
        for ((group, value) in preview["variables"]?.jsonObject ?: JsonObject(emptyMap())) {
            if (value is JsonObject) {
                for ((field, inner) in value) add("$group.$field" to inner.plainText())
            } else {
                add(group to value.plainText())
            }
        }
    }
    return PolicyAnswer(
        decision = preview.text("decision"),
        explanation = preview.text("explanation"),
        ruleName = (preview["rule"] as? JsonObject)?.text("name"),
        variables = variables,
    )
}

private fun kotlinx.serialization.json.JsonElement.plainText(): String = when (this) {
    is JsonPrimitive -> contentOrNull ?: ""
    is JsonArray -> joinToString(", ") { it.plainText() }
    is JsonObject -> entries.joinToString(", ") { "${it.key}: ${it.value.plainText()}" }
}
