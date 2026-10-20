package com.codync.android

import com.codync.android.core.HostError
import com.codync.android.core.ScreenCandidate
import java.io.IOException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.yield
import org.junit.Assert.*
import org.junit.Test

class ScreenSignalingTest {
    @Test fun `candidates in both directions wait for the installed answer`() = runBlocking {
        val h = Harness(this)
        try {
            h.start()
            h.remote.send(ScreenCandidate("ready"))
            h.signaling.awaitReady()
            h.remote.send(candidate("remote"))
            h.signaling.local(candidate("local").candidate!!, 0, "0")
            h.signaling.complete()
            yield()
            assertTrue(h.uploads.tryReceive().isFailure)
            assertTrue(h.applied.tryReceive().isFailure)
            h.signaling.answerApplied()
            assertEquals(candidate("local"), withTimeout(1000) { h.uploads.receive() })
            assertEquals("complete", withTimeout(1000) { h.uploads.receive() }.type)
            assertEquals(candidate("remote"), withTimeout(1000) { h.applied.receive() })
            assertTrue(h.failures.tryReceive().isFailure)
        } finally { h.close() }
    }

    @Test fun `each upload is acknowledged before the next starts`() = runBlocking {
        val h = Harness(this)
        try {
            h.gate = CompletableDeferred()
            h.startReady()
            h.signaling.local(candidate("first").candidate!!, 0, "0")
            h.signaling.local(candidate("second").candidate!!, 0, "0")
            h.signaling.answerApplied()
            assertEquals(candidate("first"), withTimeout(1000) { h.uploads.receive() })
            yield()
            assertTrue(h.uploads.tryReceive().isFailure)
            h.gate!!.complete(Unit)
            assertEquals(candidate("second"), withTimeout(1000) { h.uploads.receive() })
        } finally { h.close() }
    }

    @Test fun `local overflow closes signaling instead of dropping candidates`() = runBlocking {
        val h = Harness(this)
        try {
            h.startReady()
            repeat(ScreenCandidate.MAX_COUNT + 1) { h.signaling.local(candidate("$it").candidate!!, 0, "0") }
            assertTrue(withTimeout(1000) { h.failures.receive() }.message!!.contains("Too many"))
            h.signaling.answerAppliedFailure()
            assertTrue(h.uploads.tryReceive().isFailure)
        } finally { h.close() }
    }

    @Test fun `remote overflow and candidates following completion fail closed`() = runBlocking {
        for (events in listOf(List(ScreenCandidate.MAX_COUNT + 1) { candidate("$it") },
            listOf(ScreenCandidate("complete"), candidate("late")))) {
            val h = Harness(this)
            try {
                h.startReady()
                h.signaling.answerApplied()
                for (event in events) h.remote.send(event)
                assertNotNull(withTimeout(1000) { h.failures.receive() })
            } finally { h.close() }
        }
    }

    @Test fun `duplicate readiness and candidates preceding readiness fail closed`() = runBlocking {
        for (events in listOf(listOf(candidate("early")), listOf(ScreenCandidate("ready"), ScreenCandidate("ready")))) {
            val h = Harness(this)
            try {
                h.start()
                for (event in events) h.remote.send(event)
                assertNotNull(withTimeout(1000) { h.failures.receive() })
            } finally { h.close() }
        }
    }

    @Test fun `closing a replaced generation clears queues without cancelling its owner`() = runBlocking {
        val h = Harness(this)
        h.startReady()
        h.signaling.local(candidate("old").candidate!!, 0, "0")
        h.remote.send(candidate("old"))
        yield()
        h.close()
        h.signaling.answerApplied()
        yield()
        assertTrue(isActive)
        assertTrue(h.uploads.tryReceive().isFailure)
        assertTrue(h.applied.tryReceive().isFailure)
        assertTrue(h.failures.tryReceive().isFailure)
    }

    @Test fun `lost transport preserves established media but not revoked permissions`() = runBlocking {
        for (error in listOf(IOException("offline"), HostError(403, "Screen access revoked"))) {
            val h = Harness(this)
            try {
                h.connected = true
                h.startReady()
                h.remote.close(error)
                yield()
                if (error is HostError) {
                    assertEquals(error, withTimeout(1000) { h.failures.receive() })
                    assertFalse(h.signaling.isDetached)
                } else {
                    assertTrue(h.signaling.isDetached)
                    assertTrue(h.failures.tryReceive().isFailure)
                }
            } finally { h.close() }
        }
    }

    @Test fun `a fully exchanged candidate set survives transport loss before media callback`() = runBlocking {
        val h = Harness(this)
        try {
            h.startReady()
            h.signaling.answerApplied()
            h.signaling.complete()
            assertEquals("complete", withTimeout(1000) { h.uploads.receive() }.type)
            h.remote.send(ScreenCandidate("complete"))
            h.remote.close()
            yield()
            assertTrue(h.signaling.isDetached)
            assertTrue(h.failures.tryReceive().isFailure)
        } finally { h.close() }
    }

    @Test fun `transport loss during negotiation reports failure`() = runBlocking {
        val h = Harness(this)
        try {
            h.startReady()
            h.remote.close(IOException("offline"))
            assertNotNull(withTimeout(1000) { h.failures.receive() })
            assertFalse(h.signaling.isDetached)
        } finally { h.close() }
    }

    @Test fun `rejected native candidates fail even when media was connected`() = runBlocking {
        val h = Harness(this)
        try {
            h.connected = true
            h.applyFailure = IOException("Rejected remote candidate")
            h.startReady()
            h.signaling.answerApplied()
            h.remote.send(candidate("invalid-native-sdp"))
            assertEquals("Rejected remote candidate", withTimeout(1000) { h.failures.receive() }.message)
            assertFalse(h.signaling.isDetached)
        } finally { h.close() }
    }

    private class Harness(scope: CoroutineScope) {
        val remote = Channel<ScreenCandidate>(Channel.UNLIMITED)
        val uploads = Channel<ScreenCandidate>(Channel.UNLIMITED)
        val applied = Channel<ScreenCandidate>(Channel.UNLIMITED)
        val failures = Channel<Exception>(Channel.UNLIMITED)
        var connected = false
        var applyFailure: Exception? = null
        var gate: CompletableDeferred<Unit>? = null
        val signaling = ScreenSignaling({ remote.receiveAsFlow() }, { uploads.send(it); gate?.await() }, scope,
            { candidate -> applyFailure?.let { throw it }; applied.send(candidate) }, { connected }, { failures.trySend(it) })
        fun start() = signaling.start()
        suspend fun startReady() { start(); remote.send(ScreenCandidate("ready")); signaling.awaitReady() }
        fun close() { signaling.close(); remote.cancel() }
    }

    private fun ScreenSignaling.answerAppliedFailure() {
        assertThrows(IllegalArgumentException::class.java) { answerApplied() }
    }

    companion object {
        private fun candidate(id: String) = ScreenCandidate("candidate", "candidate:$id 1 UDP 1 127.0.0.1 9 typ host", 0, "0")
    }
}
