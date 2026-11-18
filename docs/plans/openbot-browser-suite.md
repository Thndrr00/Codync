# OpenBot's browser tool suite on Codync: what was taken

Reviewed 2026-11-18 against Codync `2.47.0`, from the OpenBot nightly copy at
`C:/Users/Edwin Villeda/Documents/openbot-nightly`: `src/backend/browser-tools.ts` (the tool
definitions), `src/backend/browser-host.ts` (the host behind each action), `src/backend/browser-cdp.ts`
(the page-side engine and snapshot text) and `src/backend/browser-script.ts` (the `run_script`
sandbox). Same rule as [0.69.0](openbot-0.69.0.md) and [0.72.0](openbot-0.72.0.md): behavior, field
names, defaults and text are copied and adapted to Codync's own host, and every file that holds a
piece names where it came from.

Codync already had the browser: a Codync-owned Chrome/Edge over an inherited CDP pipe with tabs
(`host/src/screen/browser_panel.rs`), used by the protected-entry flow and by the teaching recorder.
What it did not have was the model-facing suite: a bot could not open a page, read it, or act on it.
This change adds that suite on the same pipe, with a browser of the bot's own.

## Landed

| OpenBot item | Codync shape | Files |
| --- | --- | --- |
| `navigate` (an address, `back`/`forward`/`reload`, `timeoutMs`), waiting for the page before it answers | `navigate` opens the bot's browser on its first call, then `Page.navigate`, `Page.reload` or `Page.getNavigationHistory` + `Page.navigateToHistoryEntry`, polling `document.readyState` until it is `complete` twice in a row (the pipe drops CDP events, so there is no load event to wait for) | `host/src/screen/browser_tools.rs`, `host/src/screen/browser_tools/cdp.rs` |
| `snapshot`: refs, role, name, `<tag>`, value, states, bounded output | `snapshot` walks the page (and open shadow roots), numbers up to 400 visible elements, keeps the refs in the document, and prints a depth-indented tree plus the page's visible text, cut at 40000 characters | `host/src/screen/browser_tools/page.js`, `host/src/screen/browser_tools/snapshot.rs` |
| `click`, `hover`, `scroll` with trusted CDP pointer input | Resolve the ref, scroll it into view, take its centre, then `Input.dispatchMouseEvent` (`mouseMoved`, `mousePressed`, `mouseReleased`; `mouseWheel` for a scroll) | `cdp.rs` |
| `type` (replace/append, optional submit), `press` (a key or a shortcut) | Focus by ref with `Runtime.evaluate`, then `Input.insertText`; Enter for a submit; `Input.dispatchKeyEvent` for `press`, with the key table (`Enter`, `Tab`, arrows, F1–F12, one character, `Control+A` and friends) | `cdp.rs`, `host/src/screen/browser_tools/action.rs` (`Key`) |
| `select_option`, `set_checked` | The page's own `<select>`/checkbox change through the injected script, with `input` and `change` dispatched, so a framework sees the change | `page.js` |
| `wait_for` (text, url, load state, DOM quiet) | Polls a page probe every 150 ms: visible text, the address, `document.readyState`, and a `MutationObserver` that has seen nothing for half a second; answers with a fresh snapshot | `page.js`, `cdp.rs` |
| `evaluate` (`awaitPromise`, 64 KB answer) | `(0, eval)` inside the page's own context, JSON-encoded there, cut at 64000 characters; a page whose CSP forbids `eval` falls back to `Runtime.evaluate` of the expression itself | `page.js`, `cdp.rs` |
| `run_script` with the helpers bound to the same actions, at most 200 steps | `run`: the same steps as a JSON list (see below), each parsed exactly as its tool, one shared time limit, one final snapshot | `browser_tools.rs`, `action.rs` |
| Limits and readable failures | Each limit is a named constant; a page that lost an element answers `The page has no element 7; take a new snapshot.` | `action.rs`, `snapshot.rs`, `cdp.rs` |

The bot-facing surface is described in [browser tools](../features/browser-tools.md).

## Adaptations, and why

- **One bot, one browser, one tab.** OpenBot's tools take a `tabId` and manage tabs (`open`,
  `list_tabs`, `status`, `close_tab`). Codync's browser panel is the owner's, and bots must not fight
  over its active tab, so a bot gets its own Chrome/Edge process, in a browser context Codync created
  (`Target.createBrowserContext`), with one tab that every call acts on. `navigate` opens that
  browser; with none open, the other tools say so (`This bot has no browser open yet. Open a page with
  the navigate tool first.`). No tab tools, so the tab cannot be lost or multiplied by the model.
- **Refs, not OpenBot's target union.** `click`, `type`, `press`, `hover`, `scroll`, `select_option`
  and `set_checked` take a ref from the last snapshot (a number; `"7"`, `"#7"`, `"[7]"` and
  `{"ref": 7}` are accepted too). Role, text, CSS and point targets, and `nth`, are not ported: a
  snapshot plus a ref covers them, and one target shape is what a model gets right. A ref lives in the
  page, so it dies with a navigation or the next snapshot, as OpenBot's `revision` does.
- **`run` is a step list, not a JavaScript sandbox.** OpenBot runs the program in an embedded
  sandbox. Codync's host has no JavaScript engine and the scope of this change allows no new
  dependency, so `run` takes the same steps as the tools (`action` plus that tool's fields, with
  `sleep`), at most 200 of them and one time limit. A sequential sequence and a paging loop written
  out step by step are covered; arbitrary program logic, `try/catch` and `log` are not. The tool
  description says so, so a model does not write JavaScript into it.
- **No vault, cards, takeover, screenshots, recording, uploads, environments or dialogs.** Those need
  the password vault, the payment flow, the owner's hands or an Electron viewport, none of which a
  bot's own hidden browser has. `evaluate` and the snapshot text are the whole reading surface.
- **Windows, like the rest of the CDP browser.** `protected_browser` and `browser_panel` are
  `#[cfg(windows)]` (the broker is a Node child with inherited pipes and Windows creation flags), so
  `cdp.rs` is too. Off Windows the tools are not listed in the chat server at all; the argument layer,
  the snapshot formatter and their tests build and run on every platform.
- **Time limits under the MCP bridge's 30 s.** A chat tool call has 30 s in `host/src/mcp.rs`, so one
  action waits at most 20 s and a whole `run` at most 25 s; a longer wait would answer the agent with a
  dead call while the page work went on.

## Limits, in one place

| What | Limit |
| --- | --- |
| One action's wait (`timeoutMs`) | 15 s default, 20 s most |
| One `run` | 25 s, and 200 steps most |
| One `run` step's `sleep` | 25 s |
| Text one `type` enters | 20000 characters |
| JavaScript one `evaluate` takes, and answers | 64000 characters |
| Address one `navigate` opens | 2048 characters |
| Option values one `select_option` selects | 100, each 1000 characters |
| Snapshot | 400 elements, 40000 characters, text block 4000 characters |
| Page elements visited for one snapshot | 4000 |
| Scroll | 100000 pixels either way |

## Not ported, and why

- **`open`, `list_tabs`, `status`, `close_tab`, popup handling**: one bot, one tab (above).
- **`submit_secret`, `list_logins`, `list_vault_items`, `save_vault_item`, `request_vault_item`,
  `delete_vault_item`, `fill_card`, `request_link_payment`**: Codync's vault and its payment flow,
  with their own approval cards and the protected browser's secret delivery. A model driving a bot
  browser must not receive secrets, and none of this change touches `host/src/chat/teach.rs` or the
  vault.
- **`request_takeover`, `handle_dialog`, `screenshot`, `recording_start`/`recording_stop`,
  `upload_files`, `drag`, `set_environment`, `act`**: each depends on an owner-facing view, a file
  chooser, a dialog queue or a legacy action union that Codync's browser does not expose.
- **`fill_fields`, the OpenBot input repair (`browser-tool-input.ts`) and its advertised-schema
  merge**: Codync's tools take flat arguments, so a malformed call is read by one parser with one
  error, not repaired heuristically.

## Checks

`rustfmt --check --edition 2024` is clean on every changed and added host file. The host's
`cargo build`, `cargo clippy` and `cargo test` were not run in this checkout: three foreign builds
were already running there and each takes 5–10 minutes, so the suite's unit tests (ref parsing, key
parsing, the step cap, the snapshot formatter, the run's argument checks, the cut helper, the profile
folder name) are written but their run is pending on a quiet machine. The Windows CDP bridge cannot
run in the unit tests at all: it needs Chrome or Edge, so its first end-to-end check is a bot opening a
page on a Windows host. No client changed: this is a host tool surface, so the desktop, iPhone,
Android and terminal clients render nothing new.
