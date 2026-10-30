package com.codync.android

/**
 * Runs inside the mini-app page before its own scripts: sends `fetch` calls to the host through [MiniAppBridge]
 * (the WebView cannot hand request bodies to `shouldInterceptRequest`) and replaces the `_live` WebSocket with a
 * poll of the app's collections, since the host's socket is on the computer's loopback.
 */
internal fun miniAppBridgeScript(): String = """
(() => {
  const pending = new Map();
  let next = 0;
  const nativeFetch = window.fetch.bind(window);
  const nativeSocket = window.WebSocket;

  window.__codyncAnswer = (id, status, type, body) => {
    const call = pending.get(id);
    if (!call) return;
    pending.delete(id);
    const bytes = Uint8Array.from(atob(body), (c) => c.charCodeAt(0));
    call(new Response(status === 204 ? null : bytes, { status, headers: { "content-type": type } }));
  };

  window.fetch = (input, init) => {
    const request = input instanceof Request ? input : null;
    const url = new URL(request ? request.url : String(input), location.href);
    if (url.origin !== location.origin) return nativeFetch(input, init);
    const method = ((init && init.method) || (request && request.method) || "GET").toUpperCase();
    const body = init && typeof init.body === "string" ? init.body : "";
    return new Promise((resolve) => {
      const id = ++next;
      pending.set(id, resolve);
      CodyncBridge.request(id, method, url.pathname + url.search, body);
    });
  };

  window.WebSocket = function (address, protocols) {
    if (!new URL(address, location.href).pathname.endsWith("/_live")) return new nativeSocket(address, protocols);
    const socket = { readyState: 1, close() { stopped = true; }, send() {} };
    const seen = new Map();
    let stopped = false;
    let primed = false;
    const base = location.pathname.replace(/[^/]*${'$'}/, "");
    const poll = async () => {
      if (stopped) return;
      try {
        const collections = (await (await window.fetch(base + "_data")).json()).collections || [];
        for (const { collection, count, updatedAt } of collections) {
          const stamp = count + ":" + updatedAt;
          if (primed && seen.get(collection) !== stamp && socket.onmessage) {
            socket.onmessage({ data: JSON.stringify({ type: "change", collection }) });
          }
          seen.set(collection, stamp);
        }
        primed = true;
      } catch {}
      setTimeout(poll, 3000);
    };
    setTimeout(() => { if (socket.onopen) socket.onopen({}); poll(); }, 0);
    return socket;
  };
})();
"""
