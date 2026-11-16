#!/usr/bin/env node
// Codync's local web bridge: serves the web client and proxies `/api/*` and `/events` to this
// computer's host, adding the bearer token on the way. It listens on loopback only: the token never
// reaches the browser, never appears in a URL, and no other device on the network can use it.

import { createReadStream, existsSync, readFileSync, statSync } from "node:fs";
import { createServer, request } from "node:http";
import { homedir } from "node:os";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 && args[index + 1] ? args[index + 1] : fallback;
};
const dev = args.includes("--dev");
const webPort = Number(process.env.CODYNNC_WEB_PORT ?? flag("--port", 4321));
const hostPort = Number(process.env.CODYNNC_HOST_PORT ?? flag("--host-port", dev ? 19223 : 19222));
const root = resolve(flag("--root", resolve(here, "..", "out")));
const nextTarget = flag("--next", process.env.CODYNNC_NEXT ?? "");
const dataDir = process.env.CODYNNC_HOME ?? join(homedir(), dev ? ".codync-dev" : ".codync");
const tokenPath = join(dataDir, "token");

if (!Number.isInteger(webPort) || webPort < 1 || webPort > 65535) {
  console.error("The web port must be a number from 1 to 65535.");
  process.exit(1);
}

let token;
try {
  token = readFileSync(tokenPath, "utf8").trim();
} catch {
  console.error(`Can't read this computer's host token at ${tokenPath}.`);
  console.error("Start Codync once (desktop app or `codync-host serve`), then run this bridge again.");
  process.exit(1);
}
if (!token) {
  console.error(`The host token at ${tokenPath} is empty. Restart the Codync host.`);
  process.exit(1);
}

const mime = {
  ".html": "text/html; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".mp4": "video/mp4",
  ".webm": "video/webm",
  ".md": "text/markdown; charset=utf-8",
};

function respond(res, status, body, type = "text/plain; charset=utf-8") {
  res.writeHead(status, { "Content-Type": type, "Cache-Control": "no-store" });
  res.end(body);
}

function upstreamHeaders(req) {
  const headers = { ...req.headers, authorization: `Bearer ${token}`, host: `127.0.0.1:${hostPort}` };
  // The host trusts only exact loopback origins. This bridge is that origin (the browser sees the
  // bridge, the host sees a native-style call), so the page's own Origin/Referer must not travel on.
  delete headers.origin;
  delete headers.referer;
  delete headers.cookie;
  return headers;
}

/** Hop-by-hop headers belong to each connection; Node writes its own. */
function responseHeaders(headers) {
  const out = { ...headers };
  for (const name of ["connection", "keep-alive", "transfer-encoding", "upgrade", "proxy-authenticate", "proxy-authorization", "te", "trailer"]) {
    delete out[name];
  }
  return out;
}

/** `/api/<method>` and `/events` only: never the host's webhooks, channel or OAuth routes. */
function isHostPath(pathname) {
  return pathname.startsWith("/api/") || pathname === "/events";
}

function proxyHost(req, res, pathname) {
  const upstream = request(
    {
      hostname: "127.0.0.1",
      port: hostPort,
      path: req.url,
      method: req.method,
      headers: upstreamHeaders(req),
    },
    (answer) => {
      const headers = { ...responseHeaders(answer.headers), "cache-control": "no-store" };
      res.writeHead(answer.statusCode ?? 502, headers);
      answer.pipe(res);
    },
  );
  upstream.on("error", (error) => {
    if (!res.headersSent) {
      respond(res, 502, JSON.stringify({ error: `The Codync host isn't answering on 127.0.0.1:${hostPort}: ${error.message}` }), "application/json");
    } else {
      res.destroy();
    }
  });
  // A closed tab must close its SSE reader on the host too.
  res.on("close", () => {
    if (!res.writableEnded) upstream.destroy();
  });
  req.pipe(upstream);
}

function proxyNext(req, res) {
  let target;
  try {
    target = new URL(nextTarget);
  } catch {
    return respond(res, 500, "The --next address isn't a URL.");
  }
  const upstream = request(
    {
      hostname: target.hostname,
      port: target.port || 80,
      path: req.url,
      method: req.method,
      headers: { ...req.headers, host: target.host },
    },
    (answer) => {
      res.writeHead(answer.statusCode ?? 502, {
        ...responseHeaders(answer.headers),
        "x-frame-options": "DENY",
        "referrer-policy": "no-referrer",
      });
      answer.pipe(res);
    },
  );
  upstream.on("error", (error) => {
    if (!res.headersSent) respond(res, 502, `The Next.js dev server isn't answering: ${error.message}`);
    else res.destroy();
  });
  req.pipe(upstream);
}

/** Next's static export writes `/app/chat` as `app/chat.html` (and its RSC payload as `.txt`). */
function fileFor(pathname, rsc) {
  let path;
  try {
    path = decodeURIComponent(pathname);
  } catch {
    return null;
  }
  if (path.includes("\0") || path.includes("\\")) return null;
  let relative = path.replace(/^\/+/, "");
  if (!relative) relative = "index.html";
  relative = relative.replace(/\/+$/, "") || "index.html";
  // Never serve a dotfile: --root may point at a folder that holds far more than the export.
  if (relative.split("/").some((segment) => segment.startsWith("."))) return null;
  const candidates = [relative];
  if (rsc) candidates.unshift(`${relative}.txt`);
  if (!extname(relative)) candidates.push(`${relative}.html`, join(relative, "index.html"), `${relative}.txt`);
  for (const candidate of candidates) {
    const file = resolve(root, candidate);
    if (file !== root && !file.startsWith(root + sep)) continue;
    if (file.split(sep).includes("..")) continue;
    try {
      if (statSync(file).isFile()) return file;
    } catch {
      // Try the next spelling.
    }
  }
  return null;
}

function serveStatic(req, res, pathname) {
  if (req.method !== "GET" && req.method !== "HEAD") return respond(res, 405, "Method not allowed");
  const rsc = req.headers.rsc === "1" || (req.url ?? "").includes("_rsc=");
  const file = fileFor(pathname, rsc);
  if (!file) return respond(res, 404, notBuilt(pathname));
  const stat = statSync(file);
  const immutable = pathname.startsWith("/_next/static/");
  const type = rsc && file.endsWith(".txt") ? "text/x-component; charset=utf-8" : (mime[extname(file).toLowerCase()] ?? "application/octet-stream");
  res.writeHead(200, {
    "Content-Type": type,
    "Content-Length": stat.size,
    "Cache-Control": immutable ? "public, max-age=31536000, immutable" : "no-cache",
    "X-Frame-Options": "DENY",
    "Referrer-Policy": "no-referrer",
  });
  if (req.method === "HEAD") return res.end();
  createReadStream(file).pipe(res);
}

function notBuilt(pathname) {
  if (pathname !== "/" && pathname !== "/index.html") return "Not found";
  return [
    "The web client isn't built yet.",
    "",
    "  cd web",
    "  npm run build",
    "  node proxy/host-proxy.mjs",
    "",
    `Expected the export in ${root}.`,
  ].join("\n");
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? "/", `http://127.0.0.1:${webPort}`);
  if (isHostPath(url.pathname)) {
    // A custom header can't be set by another origin without a CORS preflight this bridge never
    // grants, so a form, an <img> or a sandboxed artifact can't borrow the host token. Requests
    // without it are refused before the token is attached.
    if (req.headers["x-codync-web"] !== "1") {
      return respond(res, 403, JSON.stringify({ error: "This request didn't come from the Codync web client." }), "application/json");
    }
    return proxyHost(req, res, url.pathname);
  }
  if (nextTarget) return proxyNext(req, res);
  serveStatic(req, res, url.pathname);
});

// In --next mode, Next's live reload speaks WebSocket on the same port.
if (nextTarget) {
  server.on("upgrade", (req, socket, head) => {
    let target;
    try {
      target = new URL(nextTarget);
    } catch {
      return socket.destroy();
    }
    const upstream = request({
      hostname: target.hostname,
      port: target.port || 80,
      path: req.url,
      method: req.method,
      headers: { ...req.headers, host: target.host },
    });
    upstream.on("upgrade", (answer, upstreamSocket, upstreamHead) => {
      const headers = Object.entries(answer.headers)
        .flatMap(([key, value]) => (Array.isArray(value) ? value.map((v) => `${key}: ${v}`) : value ? [`${key}: ${value}`] : []))
        .join("\r\n");
      socket.write(`HTTP/1.1 101 Switching Protocols\r\n${headers}\r\n\r\n`);
      if (upstreamHead.length) socket.write(upstreamHead);
      if (head.length) upstreamSocket.write(head);
      upstreamSocket.pipe(socket).pipe(upstreamSocket);
    });
    upstream.on("error", () => socket.destroy());
    upstream.end();
  });
}

if (!nextTarget && !existsSync(join(root, "index.html"))) {
  console.warn(`No web export in ${root}. Run \`npm run build\` in web/ first, or pass --next http://localhost:3000.`);
}

server.listen(webPort, "127.0.0.1", () => {
  console.log("");
  console.log(`  Codync web  http://localhost:${webPort}`);
  console.log(`  Host        http://127.0.0.1:${hostPort}${dev ? "  (dev)" : ""}`);
  console.log(`  Token       ${tokenPath} (stays in this process)`);
  console.log("");
  console.log("  Loopback only. Anyone who can open this address can use your bots,");
  console.log("  so don't tunnel or expose it to other devices without adding auth.");
  console.log("");
});

process.on("SIGINT", () => server.close(() => process.exit(0)));
process.on("SIGTERM", () => server.close(() => process.exit(0)));
