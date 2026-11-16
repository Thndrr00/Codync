# OpenBot 0.71.0 on Codync: what was taken

Reviewed 2026-11-15 against Codync `2.47.0`, from OpenBot nightly tag `v0.71.0`. Same rule as
[0.70.0](openbot-0.70.0.md): the license is ignored on purpose, behavior and text are copied, and
the file that holds a piece names where it came from.

0.71.0 is not a feature release the way 0.70.0 was. It adds one thing (pinch to zoom) and the rest is
four changed behaviors, one removed connector and about eighty fixes. Most of the fixes are in
machinery Codync does not have (joined servers, the Vault, the hosted product, OpenBot's own browser
and provider drivers), so this page records what applies, what Codync already does, and what stays out.

## Landed

| 0.71.0 item | Codync shape | Files |
| --- | --- | --- |
| In your own WhatsApp chat, each message from an agent no longer ends with one to three empty lines | The answer goes out trimmed at the end, paragraph breaks inside it kept | `host/src/whatsapp/outgoing.rs` (`outbound_text`) |
| Asking an agent to delete a memory that you set yourself now sends you to its Memories settings | `forget_memory` refuses a note whose current version was written by the person and tells them to delete it there | `host/src/chat/memory/beliefs.rs` |

## Added

| 0.71.0 item | Codync shape |
| --- | --- |
| Pinch to zoom under the pointer, in the main window, chat windows and browser tabs; no zoom while an agent works in a tab | Already the platform's: `apps/desktop/src/main` sets no zoom limits or factors, so Electron's own visual zoom applies to the window and its views; the chat is a view, so a pinch zooms the text as in any page. The flow canvas and the image viewer keep their own zoom, which is what the release asks for. Tab pinching does not exist because Codync's browser tab is a picture of the host's page (`views/browser/BrowserWindow.tsx`), zoomed by the host's own page zoom. Not run here: this machine has no touchpad or touch screen, so the behavior is the platform default as read in the code, not a tested gesture. |

## Already here, verified in the code

| 0.71.0 item | Where Codync already does it |
| --- | --- |
| An agent that repeated the same failed step goes on until you stop it; now it stops after 5 | Stricter here: `host/src/agent/bot/stall.rs` stops the turn after **3** identical failed tool calls in a row (`SAME_ERROR_STRIKES`) and says which call it was. |
| A message to an Antigravity agent now joins the running turn | Nothing to add: any backend whose ACP advertises `_session/steering` steers (`host/src/agent/bot/steer.rs` checks `sessions.steering`), and the note added for 0.69.0 already covers Antigravity. Whether a given CLI version advertises it is the CLI's own business. |
| Stopping an agent also cancels the requests it left waiting in a teammate's queue | `host/src/chat/team/blocking.rs`: "Stopping you cancels your pending asks", and dropping the caller cancels only its own request. |
| A routine whose task says to stop when the work is done pauses itself | The routine tools already let an agent turn off its own routine (`set_routine_enabled` in `host/src/routines/mcp.rs`), and another bot's routine is refused. |

## Not ported, and why

- **Hosted servers on Leaseweb, Starter pricing, the payment page, the yearly price**: the hosted
  product, not the app; Codync's cloud is its own (`cloud/`).
- **MongoDB Atlas removed from the Apps catalog**: Codync's catalog is imported from its own lists
  (`tools/import-openbot-catalog.mjs`); a connector it does not carry has nothing to remove. Every
  other connector fix (YNAB, TradingView, Proton Pass subdomains, Bitwarden) belongs to its
  connector's own change.
- **Browser and driver fixes** (a tab that nobody sees slowing down, the language header a site sets,
  a click on a link that moves, a page unloaded behind the agent's back, a form field in a frame, slots
  of a web component, cookie handling for another site's frame, transcripts of long videos): these are
  inside OpenBot's embedded browser and its page driver. Codync drives the person's browser and its
  own protected browser through a different layer (`host/src/screen/protected_browser.rs`), so none of
  these patches apply as written; each one that shows up here becomes its own fix.
- **Provider-driver fixes** (Pi `switch_session` timeouts, a slow CLI's 10 s version check, Codex
  credits and plugin prompts, Claude's monthly-limit message, OpenCode sessions and image limits, Kimi
  sign-in, ClinePass, an OpenRouter answer-token cap, a Claude model that moves back to the default,
  Gemini capacity and long commands, Grok's tool names and moved workspaces): they live in OpenBot's
  own client per provider. Codync speaks ACP to the CLI and reads its reports, so the fix is the CLI's
  version, not Codync's code.
- **Joined-server fixes** (install card, provider change while an agent works, channel copies of
  messages, a Slack or Discord workspace that belongs to another server, a server's remote desktop,
  moving a server, `socat` and AppArmor on Ubuntu 26.04, the moved server's `/srv` folders): joined
  server work, as in [0.69.0](openbot-0.69.0.md).
- **Channel-task fixes** (why a request waits, a lead that keeps its place in the queue, a result that
  arrives after a background command, stopping sub-tasks): Codync has no channel tasks; its paused
  turn and routine notices stand in, as recorded in [mobile parity](openbot-mobile-parity.md).
- **Vault fixes** (a key folder, a subdomain's login, an API key's old path, a wrong login id): Codync
  has no Vault.
- **Voice fixes** (a reply read above 1× that pauses, noise that keeps a spoken reply paused, a model
  that knows the language before you choose one, proxy settings for voice downloads): Codync's speech
  engine is the platform's on both phones and its own chunking on the desktop
  (`views/call/voice.ts`), so these are written against OpenBot's engine. They are candidates for a
  look of their own, not copies.
- **The canvas**: dragging a step or a note by its text, the caret where a click lands, and Stop for
  the routine in focus. Codync's flow canvas has its own pointer handling (`lib/routine-flow.ts`,
  `core/RoutineFlow.kt`) and its run controls live in the routine's own panel; matching OpenBot's
  pointer rules there is a change of its own, and a Stop that ends a run belongs next to Run now, which
  is where the desktop and Android panels already keep it.
- **A note path from a notes app written as a file link**, **the image viewer's sharpness at a display
  scale above 100 %**, **resizing the right panel when no pane is open**, **the empty WhatsApp lines of
  a theme**: each is a small UI or copy fix whose Codync counterpart needs its own read; none is taken
  here.

## Checks

No code of this release is ported yet, so there is nothing of its own to check. The branch's own
gates are recorded in [0.70.0](openbot-0.70.0.md#checks) and in the commit that carries the 0.69.0 and
0.70.0 follow-ups.
