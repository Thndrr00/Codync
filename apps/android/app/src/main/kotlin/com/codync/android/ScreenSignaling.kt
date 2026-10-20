package com.codync.android

import com.codync.android.core.EncryptedChannel
import com.codync.android.core.HostError
import com.codync.android.core.ScreenCandidate
import com.codync.android.core.WireJson
import java.io.IOException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.put

/** One generation's bounded ICE exchange. All public methods and callbacks run on the session's main scope. */
internal class ScreenSignaling(
    private val subscribe: () -> Flow<ScreenCandidate>,
    private val upload: suspend (ScreenCandidate) -> Unit,
    parentScope: CoroutineScope,
    private val applyCandidate: suspend (ScreenCandidate) -> Unit,
    private val isMediaConnected: () -> Boolean,
    private val onFailure: (Exception) -> Unit,
) {
    constructor(channel: EncryptedChannel, session: String, parentScope: CoroutineScope,
        applyCandidate: suspend (ScreenCandidate) -> Unit, isMediaConnected: () -> Boolean,
        onFailure: (Exception) -> Unit) : this(
        subscribe = { channel.subscribe("screenCandidates", buildJsonObject { put("session", session) })
            .map { WireJson.decodeFromJsonElement<ScreenCandidate>(it) } },
        upload = { candidate -> channel.call("screenCandidate", buildJsonObject {
            put("session", session)
            put("candidate", WireJson.encodeToJsonElement(candidate))
        }); Unit }, parentScope, applyCandidate, isMediaConnected, onFailure)

    private enum class State { Active, Detached, Closed }
    private val scope = CoroutineScope(parentScope.coroutineContext + SupervisorJob(parentScope.coroutineContext[Job]))
    private val ready = CompletableDeferred<Unit>()
    private val answered = CompletableDeferred<Unit>()
    private val outgoing = Channel<ScreenCandidate>(ScreenCandidate.MAX_COUNT + 1)
    private var state = State.Active
    private var started = false
    private var remoteReady = false
    private var localCount = 0
    private var remoteCount = 0
    private var queuedComplete = false
    private var localComplete = false
    private var remoteComplete = false
    private var failure: Exception? = null

    val isDetached: Boolean get() = state == State.Detached

    fun start() {
        check(!started) { "Screen signaling already started." }
        check(state == State.Active) { "Screen signaling closed." }
        started = true
        scope.launch {
            try {
                subscribe().collect { receive(it) }
                transportEnded(IOException("Screen signaling disconnected."))
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { handle(error) }
        }
        scope.launch {
            try {
                answered.await()
                for (candidate in outgoing) {
                    upload(candidate)
                    if (candidate.type == "complete") localComplete = true
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { handle(error) }
        }
    }

    /** Must finish before screenOffer: the host reserves one private receiver per viewer. */
    suspend fun awaitReady() {
        check(started) { "Start screen signaling before waiting for it." }
        if (withTimeoutOrNull(10_000) { ready.await() } == null) {
            val error = IOException("Screen signaling didn't become ready. Reconnect and try again.")
            fail(error)
            throw error
        }
        failure?.let { throw it }
        check(state == State.Active) { "Screen signaling closed." }
    }

    fun local(candidate: String, index: Int, mid: String?) = enqueue(ScreenCandidate("candidate", candidate, index, mid))

    fun complete() = enqueue(ScreenCandidate("complete"))

    /** Called only once setRemoteDescription has successfully installed the answer. */
    fun answerApplied() {
        failure?.let { throw it }
        if (state == State.Active) answered.complete(Unit)
    }

    private fun enqueue(candidate: ScreenCandidate) {
        if (state != State.Active) return
        try {
            candidate.validate()
            require(!queuedComplete) { "Screen candidates already complete." }
            if (candidate.type == "complete") queuedComplete = true
            else {
                localCount += 1
                require(localCount <= ScreenCandidate.MAX_COUNT) { "Too many screen candidates." }
            }
            require(outgoing.trySend(candidate).isSuccess) { "Screen candidate queue overflow." }
        } catch (error: Exception) { fail(error) }
    }

    private suspend fun receive(candidate: ScreenCandidate) {
        candidate.validate()
        when (candidate.type) {
            "ready" -> {
                require(!remoteReady) { "Screen candidates already subscribed." }
                remoteReady = true
                ready.complete(Unit)
            }
            "error" -> throw SignalingFailure(candidate.message ?: "Screen signaling failed.")
            "candidate", "complete" -> {
                require(remoteReady && !remoteComplete) { "Screen candidates arrived out of order." }
                if (candidate.type == "complete") remoteComplete = true
                else {
                    remoteCount += 1
                    require(remoteCount <= ScreenCandidate.MAX_COUNT) { "Too many remote screen candidates." }
                }
                answered.await()
                if (state == State.Active && candidate.type == "candidate") {
                    try { applyCandidate(candidate) }
                    catch (cancelled: CancellationException) { throw cancelled }
                    catch (error: Exception) { throw SignalingFailure(error.message ?: "Couldn't apply a screen candidate.") }
                }
            }
        }
    }

    private fun handle(error: Exception) {
        when {
            error is SignalingFailure -> fail(error)
            error is HostError && error.status < 500 -> fail(error)
            error is IOException -> transportEnded(error)
            else -> fail(error)
        }
    }

    private fun transportEnded(error: Exception) {
        if (state != State.Active) return
        if (isMediaConnected() || (localComplete && remoteComplete)) {
            state = State.Detached
            finish()
        } else fail(error)
    }

    private fun fail(error: Exception) {
        if (state != State.Active) return
        failure = error
        state = State.Closed
        ready.completeExceptionally(error)
        finish()
        onFailure(error)
    }

    fun close() {
        if (state == State.Closed) return
        state = State.Closed
        finish()
    }

    private fun finish() {
        ready.cancel()
        answered.cancel()
        outgoing.cancel()
        scope.cancel()
    }

    private class SignalingFailure(message: String) : IOException(message)
}
