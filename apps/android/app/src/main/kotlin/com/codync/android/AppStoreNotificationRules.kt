package com.codync.android

import kotlinx.serialization.json.*

/** Which alerts reach the phone, and the quiet hours around them (host `notificationRules`). */
internal data class NotificationRules(
    val done: Boolean,
    val needsInput: Boolean,
    val failed: Boolean,
    val paused: Boolean,
    val quietStart: Int,
    val quietEnd: Int,
    val timezone: String?,
)

private fun rulesOf(json: JsonObject): NotificationRules = NotificationRules(
    done = json.boolean("done") ?: true,
    needsInput = json.boolean("needsInput") ?: true,
    failed = json.boolean("failed") ?: true,
    paused = json.boolean("paused") ?: true,
    quietStart = json.int("quietStart") ?: 0,
    quietEnd = json.int("quietEnd") ?: 0,
    timezone = json.text("timezone"),
)

private fun JsonObject.boolean(key: String): Boolean? = (this[key] as? JsonPrimitive)?.booleanOrNull
private fun JsonObject.int(key: String): Int? = (this[key] as? JsonPrimitive)?.intOrNull
private fun JsonObject.text(key: String): String? =
    (this[key] as? JsonPrimitive)?.takeIf { it.isString }?.content

internal suspend fun AppStore.notificationRules(): NotificationRules =
    rulesOf(hostCall("notificationRules", buildJsonObject {}).jsonObject["rules"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.setNotificationRules(rules: NotificationRules): NotificationRules =
    rulesOf(hostCall("setNotificationRules", buildJsonObject {
        put("rules", buildJsonObject {
            put("done", rules.done)
            put("needsInput", rules.needsInput)
            put("failed", rules.failed)
            put("paused", rules.paused)
            put("quietStart", rules.quietStart)
            put("quietEnd", rules.quietEnd)
            if (rules.timezone != null) put("timezone", rules.timezone) else put("timezone", JsonNull)
        })
    }).jsonObject["rules"]?.jsonObject ?: buildJsonObject {})
