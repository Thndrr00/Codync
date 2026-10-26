package com.codync.android

import com.codync.android.core.WireJson
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull

/** Delivered, or permanently rejected. Timeouts, throttling, redirects and server failures remain queued. */
internal fun analyticsDeliveryAccepted(status: Int): Boolean =
    status in 200..299 || (status in 400..499 && status != 408 && status != 429)

/** Persist immutable event identities before sending; acknowledgements only remove their own batch. Main thread. */
internal class AnalyticsQueue(
    read: () -> String?,
    private val write: (String) -> Boolean,
    restore: Boolean,
    private val capacity: Int = 500,
) {
    private val pending = mutableListOf<JsonObject>()
    val size: Int get() = pending.size

    init {
        if (restore) {
            val saved = runCatching { WireJson.parseToJsonElement(read().orEmpty()) as? JsonArray }.getOrNull()
            pending.addAll(saved.orEmpty().filterIsInstance<JsonObject>().filter {
                (it["uuid"] as? JsonPrimitive)?.contentOrNull != null &&
                    (it["event"] as? JsonPrimitive)?.contentOrNull != null
            }.takeLast(capacity))
        } else clear()
    }

    fun add(event: JsonObject): Boolean {
        pending += event
        while (pending.size > capacity) pending.removeAt(0)
        return persist()
    }

    fun batch(limit: Int): List<JsonObject> = if (persist()) pending.take(limit) else emptyList()

    fun acknowledge(batch: List<JsonObject>) {
        val ids = batch.map { it["uuid"] }.toSet()
        pending.removeAll { it["uuid"] in ids }
        persist()
    }

    fun clear() { pending.clear(); persist() }

    private fun persist(): Boolean = write(JsonArray(pending).toString())
}

/** The network thread must register its cancellation before dispatch; stale consent never admits a request. */
internal class AnalyticsConsent(initial: Boolean) {
    private val lock = Any()
    private var allowed = initial
    private var generation = 0
    private var activeGeneration: Int? = null
    private var cancelActive: (() -> Unit)? = null

    val version: Int get() = synchronized(lock) { generation }

    fun change(on: Boolean) {
        val cancel = synchronized(lock) {
            generation++
            allowed = on
            val previous = cancelActive
            cancelActive = null
            activeGeneration = null
            previous
        }
        cancel?.invoke()
    }

    fun allows(version: Int): Boolean = synchronized(lock) { allowed && generation == version }

    fun register(version: Int, cancel: () -> Unit): Boolean = synchronized(lock) {
        if (!allowed || generation != version) return@synchronized false
        activeGeneration = version
        cancelActive = cancel
        true
    }

    fun finished(version: Int) = synchronized(lock) {
        if (activeGeneration == version) {
            activeGeneration = null
            cancelActive = null
        }
    }
}
