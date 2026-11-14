package com.codync.android

import com.codync.android.core.*
import kotlinx.coroutines.flow.update
import kotlinx.serialization.json.*

// Routine flows (docs/features/routine-flows.md): a bot's canvas as the host answers it, and the intents that change it. The
// host validates every change and runs the flows; this keeps the canvas, the request in flight and the error.

/** The `routineFlow` events in a batch: the canvases of those bots changed, so one on screen loads again. */
internal fun AppStore.receiveRoutineFlows(events: List<MirrorEvent>) {
    val changed = events.filterIsInstance<MirrorEvent.RoutineFlow>()
    if (changed.isEmpty()) return
    mutableState.update { state ->
        state.copy(flowRevs = changed.fold(state.flowRevs) { revs, event ->
            event.botIds.fold(revs) { acc, id -> acc + (id to maxOf(event.rev, acc[id] ?: 0)) }
        })
    }
}

private suspend fun AppStore.receiveFlow(botId: String) {
    val response = ready().call("routineFlowCanvas", body("botId" to botId))
    val canvas = WireJson.decodeFromJsonElement<FlowCanvas>(response)
    mutableState.update { it.copy(flows = it.flows + (botId to canvas)) }
}

/** Loads a bot's canvas (also when a `routineFlow` event moved its rev). */
fun AppStore.loadFlow(botId: String) = action("flow:$botId") { receiveFlow(botId) }

/** Runs one change on the host, then loads the canvas again. `done` gets the host's answer. */
private fun AppStore.flowChange(botId: String, method: String, payload: JsonObject, done: (JsonObject) -> Unit = {}) =
    action("flow:$botId") {
        val answer = ready().call(method, payload).jsonObject
        receiveFlow(botId)
        done(answer)
    }

/** A node moved: shown where it was dropped at once, then saved. */
fun AppStore.moveFlowNode(botId: String, nodeKey: String, x: Double, y: Double) {
    val place = FlowPosition(nodeKey, RoutineFlow.clampCoordinate(x), RoutineFlow.clampCoordinate(y))
    mutableState.update { state ->
        val canvas = state.flows[botId] ?: return@update state
        state.copy(flows = state.flows + (botId to canvas.copy(positions = canvas.positions.filterNot { it.nodeKey == nodeKey } + place)))
    }
    flowChange(botId, "saveRoutineFlowPosition", buildJsonObject {
        put("botId", botId); put("nodeKey", place.nodeKey); put("x", place.x); put("y", place.y)
    })
}

fun AppStore.placeFlowBot(botId: String, placed: String, x: Double, y: Double) =
    flowChange(botId, "placeRoutineFlowBot", buildJsonObject {
        put("botId", botId); put("placedBotId", placed)
        put("x", RoutineFlow.clampCoordinate(x)); put("y", RoutineFlow.clampCoordinate(y))
    })

fun AppStore.removeFlowNode(botId: String, nodeKey: String) =
    flowChange(botId, "removeRoutineFlowPosition", buildJsonObject { put("botId", botId); put("nodeKey", nodeKey) })

fun AppStore.connectFlow(botId: String, routineId: String, from: String, to: String, instruction: String = "", linked: (FlowLink) -> Unit = {}) =
    flowChange(botId, "connectRoutineFlow", buildJsonObject {
        put("routineId", routineId); put("fromBotId", from); put("toBotId", to); put("instruction", instruction)
    }) { answer -> linked(WireJson.decodeFromJsonElement(answer.getValue("link"))) }

/** Saves the instruction and the model the step runs on; a null model is the next bot's own. */
fun AppStore.updateFlowLink(botId: String, linkId: String, instruction: String, model: String?, modelOptions: Map<String, String>) =
    flowChange(botId, "updateRoutineFlowLink", buildJsonObject {
        put("linkId", linkId); put("instruction", instruction); put("model", model)
        put("modelOptions", buildJsonObject { for ((id, value) in modelOptions) put(id, value) })
    })

fun AppStore.disconnectFlow(botId: String, linkId: String) =
    flowChange(botId, "disconnectRoutineFlow", buildJsonObject { put("linkId", linkId) })

/** Adds a picture or clip step to a routine, at its model's first settings. */
fun AppStore.addFlowMedia(botId: String, routineId: String, kind: String, added: (FlowMedia) -> Unit = {}) =
    flowChange(botId, "addRoutineFlowMedia", buildJsonObject { put("routineId", routineId); put("kind", kind) }) { answer ->
        added(WireJson.decodeFromJsonElement(answer.getValue("media")))
    }

/** Saves a step's description, model and settings. */
fun AppStore.updateFlowMedia(botId: String, mediaId: String, settings: MediaChoice, prompt: String) =
    flowChange(botId, "updateRoutineFlowMedia", buildJsonObject {
        put("mediaId", mediaId); put("prompt", prompt); put("model", settings.model); put("aspect", settings.aspect)
        put("duration", settings.duration); put("resolution", settings.resolution); put("transparent", settings.transparent)
        put("voice", settings.voice)
    })

fun AppStore.removeFlowMedia(botId: String, mediaId: String) =
    flowChange(botId, "removeRoutineFlowMedia", buildJsonObject { put("mediaId", mediaId) })

/** Starts the flow with a sample reply, without running the routine. */
fun AppStore.testFlow(botId: String, routineId: String) =
    flowChange(botId, "testRoutineFlow", buildJsonObject { put("botId", botId); put("routineId", routineId) })

fun AppStore.setFlowRoutineEnabled(botId: String, routineId: String, enabled: Boolean) =
    flowChange(botId, "setRoutineEnabled", buildJsonObject { put("botId", botId); put("id", routineId); put("enabled", enabled) })
