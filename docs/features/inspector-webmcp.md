# Inspector and WebMCP

Two optional development surfaces from card K15. Both are off by default and neither adds a dependency.

## Inspector (desktop, development only)

A panel over the chat window with what the host is doing right now:

- **Recent events** — the newest host events this computer received, newest last, each folded open to its
  fields.
- **Active turns** — every bot working now, with the computer it runs on and its status.
- **View state** — the saved K4 state of each view the window reports (`chat:<botId>`, `routine:<botId>`).

Open it with **⌘/Ctrl + Shift + .** (writes `codync.inspector` in local storage) or add `?inspector=1` to the
window URL. It is mounted only when `import.meta.env.DEV` is true, so a packaged release has no panel, no
toggle and no event recording (`process.env.NODE_ENV` is production in the main process).

**Redaction** (`apps/desktop/src/shared/inspector.ts`) runs before an event is stored, not when it is
drawn: keys matching token, secret, password, authorization, cookie, credential or API/private key become
`[redacted]`; prompt, body, message, transcript, command, cwd and path fields become `[redacted]`; URLs lose
their query and fragment (the capability token of a mini-app link never reaches the panel); strings are cut
at 120 characters, lists at 20 items, nesting at four levels, and the log holds the newest 200 events. A host
event that arrives malformed still shows its type and shape, which is how a bad event is diagnosed: open the
Inspector, find the event, and read the field that the client ignored. Nothing leaves the computer.

## WebMCP

A host-served mini-app page (the `/apps/{bot}/{slug}/{token}/` pages that the host itself serves) can ask
Codync to perform one of three **declared** actions:

| Action | Arguments | What it does |
|---|---|---|
| `open_chat` | `botId` | Brings that bot's chat on screen. |
| `show_artifact` | `artifactId` | Opens that bot's artifact in the chat's artifact tab. |
| `start_call` | `botId` | Starts a voice call with that bot. |

The whole allow list lives in `host/src/webmcp/`, is returned by `GET …/_webmcp`, and is registered with the
browser's `navigator.modelContext` when it exists; a call for an action the person hasn't granted is refused.
The page calls `POST …/_webmcp` with `{action, arguments}` or uses `window.codync.actions()` /
`window.codync.act(action, args)` from the injected `_codync.js`.

**Permission is explicit and per app.** `webMcpState` returns the declared actions and whether each is
granted for that app; `setWebMcpGrant` replaces the grant. Both host methods are decided at the computer
(the app panel in the desktop app — **Browser actions** with one switch per action). A new app starts with
no grant, and a call without one is refused. Grants are stored in the host's `kv` (`webmcp.v1.<bot>.<slug>`)
and survive restarts.

**Only host-served pages.** The endpoint exists only under the app runtime, checks the capability link like
every other app request, and refuses a `POST` whose `Origin` is a website: it accepts the host's own loopback
origin, `null` (the opaque origin of the sandboxed mini-app frame) or no header (a same-origin call). A
website in the browser panel still needs the app's capability link, and a phone's `appRequest` bridge cannot
use the endpoint at all.

**Never credentials or arbitrary host calls.** The host validates the named bot and artifact against its
store, emits `{"type":"webMcpAction","action","arguments"}` with only the ids the page already supplied, and
the apps do the navigation with their own UI state. The page never receives a token, a file path, a host
method name or a database value; adding an action is a code change with a schema, a grant and a test.

## Trying it

1. Run the desktop app unpackaged (`npm run dev`), then press ⌘/Ctrl + Shift + .; the Inspector opens over
   the chat. Trigger a turn and watch its events and active turn appear.
2. Open a bot-built mini-app, turn on **Browser actions**, then in the page call
   `codync.act('open_chat', { botId: '<a bot id>' })`. The chat view switches to that bot.
3. Turn the switch off and repeat: the call answers 403 and nothing moves.
