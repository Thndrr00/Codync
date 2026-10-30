package com.codync.android

import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import com.codync.android.core.MINI_APP_ORIGIN
import com.codync.android.core.MiniAppAddress
import com.codync.android.core.MiniAppAnswer
import com.codync.android.core.miniAppPolicy
import java.io.ByteArrayInputStream
import java.util.Base64
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonPrimitive

private const val BRIDGE_PATH = "/_codync_bridge.js"
private val headTag = Regex("<head[^>]*>", RegexOption.IGNORE_CASE)

/** Serves a mini-app to a phone WebView by asking the host for every request (pages, files, data, tells). */
internal class MiniAppWebClient(
    private val address: MiniAppAddress,
    private val store: AppStore,
    private val onFailure: (String) -> Unit,
) : WebViewClient() {
    override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? {
        val url = request.url
        if (!url.toString().startsWith(MINI_APP_ORIGIN)) return null
        val path = url.path.orEmpty()
        if (path == BRIDGE_PATH) return response(200, "text/javascript", miniAppBridgeScript().toByteArray(), "")
        val relative = address.relativePath(path) ?: return response(403, "text/plain", "Outside this mini-app".toByteArray(), "")
        if (request.method != "GET") return response(405, "text/plain", ByteArray(0), "")
        val target = url.encodedQuery?.let { "$relative?$it" } ?: relative
        return try {
            val answer = runBlocking { store.appRequest(address, "GET", target) }
            val bytes = Base64.getDecoder().decode(answer.body)
            val isPage = answer.contentType.startsWith("text/html")
            response(answer.status, answer.contentType, if (isPage) withBridge(bytes) else bytes, answer.csp)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (error: Exception) {
            if (request.isForMainFrame) onFailure(error.message ?: "Could not reach the computer.")
            response(502, "text/plain", ByteArray(0), "")
        }
    }

    private fun withBridge(html: ByteArray): ByteArray {
        val text = String(html, Charsets.UTF_8)
        val tag = "<script src=\"$BRIDGE_PATH\"></script>"
        val head = headTag.find(text)
        return (if (head != null) text.replaceRange(head.range.last + 1, head.range.last + 1, tag) else tag + text).toByteArray()
    }

    private fun response(status: Int, contentType: String, body: ByteArray, csp: String): WebResourceResponse {
        val mime = contentType.substringBefore(';').trim().ifEmpty { "application/octet-stream" }
        val headers = mutableMapOf("Cache-Control" to "no-store")
        miniAppPolicy(csp).takeIf { it.isNotEmpty() }?.let { headers["Content-Security-Policy"] = it }
        return WebResourceResponse(mime, "utf-8", status, if (status < 400) "OK" else "Error", headers, ByteArrayInputStream(body))
    }
}

/** What the page's `fetch` calls reach: it relays a request to the host and returns the answer to the page. */
internal class MiniAppBridge(
    private val address: MiniAppAddress,
    private val store: AppStore,
    private val scope: CoroutineScope,
    private val view: WebView,
) {
    @JavascriptInterface fun request(id: Int, method: String, path: String, body: String) {
        scope.launch {
            val answer = try {
                val relative = address.relativePath(path.substringBefore('?')) ?: error("Outside this mini-app")
                val query = path.substringAfter('?', "")
                store.appRequest(address, method, if (query.isEmpty()) relative else "$relative?$query", body)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                MiniAppAnswer(502, "application/json", "", Base64.getEncoder().encodeToString(
                    """{"error":${JsonPrimitive(error.message ?: "Could not reach the computer.")}}""".toByteArray()))
            }
            val reply = "window.__codyncAnswer($id,${answer.status},${JsonPrimitive(answer.contentType)},${JsonPrimitive(answer.body)})"
            view.post { view.evaluateJavascript(reply, null) }
        }
    }
}
