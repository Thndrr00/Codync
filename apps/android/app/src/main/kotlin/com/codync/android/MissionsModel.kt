package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.codync.android.core.Mission
import com.codync.android.core.MissionStatus
import com.codync.android.core.MissionStep
import com.codync.android.core.MissionStepStatus
import com.codync.android.core.WireJson
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

/** One bot's missions: the list and every move a person can make on it. The host owns the rules. */
internal class MissionsModel(
    private val botId: String, private val store: AppStore,
    private val scope: CoroutineScope, private val generation: Int,
) {
    var missions by mutableStateOf<List<Mission>>(emptyList()); private set
    var finishedShown by mutableStateOf(false); private set
    var loading by mutableStateOf(false); private set
    var busy by mutableStateOf(false); private set
    var error by mutableStateOf<String?>(null); private set
    private var listing = 0

    val open: List<Mission> get() = missions.filter { !it.status.returned }
    val finished: List<Mission> get() = missions.filter { it.status.returned }

    private fun active() = generation == store.state.value.generation && !store.state.value.loading

    private suspend fun call(method: String, values: JsonObject) =
        store.hostQuery(method, values, expectedGeneration = generation).jsonObject

    private fun task(block: suspend () -> Unit) = scope.launch {
        try { block() }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "That did not work." }
    }

    fun watch() = task { load() }

    fun refresh() = task { load() }

    fun toggleFinished() {
        finishedShown = !finishedShown
        task { load() }
    }

    private suspend fun load() {
        if (!active()) return
        val sequence = ++listing
        loading = true
        try {
            val open = call("missions", buildJsonObject { put("botId", botId); put("open", true) })
            val all = if (finishedShown) call("missions", buildJsonObject { put("botId", botId) }) else null
            if (sequence != listing || !active()) return
            val merged = LinkedHashMap<String, Mission>()
            for (element in open.getValue("missions").jsonArray) {
                WireJson.decodeFromJsonElement(Mission.serializer(), element).let { merged[it.id] = it }
            }
            all?.let { every ->
                for (element in every.getValue("missions").jsonArray) {
                    WireJson.decodeFromJsonElement(Mission.serializer(), element).let { merged[it.id] = it }
                }
            }
            missions = merged.values.sortedByDescending { it.updatedAt }
            error = null
        } finally {
            loading = false
        }
    }

    /** Runs one change and reads the list again; the message a refusal carries is the page's error. */
    private fun change(method: String, id: String, values: JsonObject) = task {
        busy = true
        error = null
        try {
            call(method, JsonObject(values + ("id" to JsonPrimitive(id))))
            load()
        } finally {
            busy = false
        }
    }

    fun create(title: String, goal: String, steps: List<String>) {
        val trimmed = title.trim()
        if (trimmed.isEmpty()) return
        task {
            busy = true
            error = null
            try {
                call("createMission", buildJsonObject {
                    put("botId", botId)
                    put("title", trimmed)
                    put("goal", goal.trim())
                    putJsonArray("steps") { steps.filter { it.isNotBlank() }.forEach { step -> add(step.trim()) } }
                })
                load()
            } finally {
                busy = false
            }
        }
    }

    fun move(mission: Mission, to: MissionStatus, stopReason: String? = null) =
        change("updateMission", mission.id, buildJsonObject {
            put("status", to.name.lowercase())
            stopReason?.let { reason -> put("stopReason", reason) }
        })

    fun checkpoint(mission: Mission, text: String) =
        change("updateMission", mission.id, buildJsonObject { put("checkpoint", text) })

    fun addStep(mission: Mission, text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        change("missionStep", mission.id, buildJsonObject { put("text", trimmed) })
    }

    fun setStep(mission: Mission, step: MissionStep, status: MissionStepStatus) =
        change("missionStep", mission.id, buildJsonObject {
            put("stepId", step.id)
            put("status", status.name.lowercase())
        })
}
