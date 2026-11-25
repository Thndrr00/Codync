# Browser tools

A bot with **Use the computer** on and its browser permission gets fourteen browser tools from the
built-in `chat` MCP server (`host/src/mcp.rs`): `navigate`, `snapshot`, `screenshot`, `click`, `type`,
`press`, `hover`, `drag`, `scroll`, `select_option`, `set_checked`, `wait_for`, `evaluate` and `run`. They drive a
Chrome or Edge browser of the bot's own, so a routine can sign in to a site, fill a form, read a
result or check a page, and the owner can watch it happen in a real window.

The suite and its limits are ported from OpenBot's browser tools; what was taken and what was left
behind is in [OpenBot's browser tool suite on Codync](../plans/openbot-browser-suite.md).

## Where it runs, and who may use it

- The host runs the browser (`host/src/screen/browser_tools.rs`, and `browser_tools/cdp.rs` on
  Windows) on the same inherited CDP pipe as the browser panel and protected entry
  (`host/src/screen/protected_browser/pipe.rs`), so there is one browser stack, not two.
- A call needs the bot's computer access with the browser permission, the same gate as the computer
  tools (`hub.computer.begin(…, Capability::Browser)`). Turning computer use or the browser permission
  off revokes the tools immediately, and a protected entry in progress pauses them.
- The bridge needs Windows, like the rest of Codync's CDP browser (the broker is a Node child with
  inherited pipes). Off Windows the tools are not listed, so a model never sees a tool that cannot
  work.
- Nothing in the apps changed: these are host tools an agent calls, and the browser window they open
  is a plain Chrome or Edge window, not a Codync screen.

## The bot's browser

The first `navigate` starts one Chrome or Edge process in the bot's own profile
(`<data>/protected-browsers/bots/<bot>`), creates a browser context Codync owns
(`Target.createBrowserContext`), and opens one tab. Every other tool acts on that tab; `navigate`
again moves it. If the page closes its own tab, the next call opens a new one. The browser is
incognito at the process level, its profile is removed when it is stopped, and a browser that stops
answering is closed rather than reused, because a cancelled CDP read leaves the pipe unsure where the
next reply starts.

With no browser open, everything but `navigate` answers: *This bot has no browser open yet. Open a page
with the navigate tool first.*

## Watching a tab live and taking control

The owner can open **Watch this bot's browser tab** from the bot's Computer panel to see the tab the
tools are driving, without opening a second browser of its own. The host
attaches to that tab over the same CDP pipe (`browser_tools/live.rs` and `cdp.rs`), captures at most
two bounded JPEG frames a second together with the tab's title and address, and stops when the sheet
closes or nobody is watching. Until someone presses **Take control** the view is read-only; while
control is held the desk's pointer and key events go down the same `Input.dispatchMouseEvent` and
`Input.dispatchKeyEvent` path the `click` and `press` tools use, and a badge shows that a person is in
control. **Release** gives the tab back. Watching and controlling use the same computer-access and
browser-permission gate as every other tool, re-checked on each frame and input; a protected entry or
a turned-off permission stops the view, and the methods are local to this computer. Frames, titles,
addresses and input values are never logged or sent to analytics.

A thread uses its bot's tabs, not a second store (OpenBot 0.75.0): the CDP
sessions in `browser_tools/cdp.rs` are keyed by bot id, and the live-tab view in
`browser_tools/live.rs` attaches to the same slot. A chat and a thread of the
same bot therefore share one browser context, profile and tab; no separate
per-thread tab map was needed.

Unused tabs freeze after five minutes without CDP activity (OpenBot 0.75.0): a bounded watcher in
`browser_tools/idle.rs` + `cdp.rs` calls `Page.setWebLifecycleState('frozen')` on the bot's idle tab,
and wakes it with `'active'` on the next live frame, input event or tool call. It never freezes a tab
someone is watching live, one held by a tool call, or one whose bot has a running turn; frames and
input keep working.

## Refs and snapshots

`snapshot` numbers the elements of the page and prints one line each:

```
url: https://example.com/login
title: Sign in
1 textbox "Email" <input> value="ada@example.com"
  2 checkbox "Remember me" <input> checked
3 button "Sign in" <button>
text: Welcome back
```

Each line is the ref, the role, the accessible name, the tag, then the value and states when the
element has them; the indentation is the depth. Afterwards comes the page's own visible text. Only
visible elements are numbered — 400 of them at most, and the whole answer is cut at 40000 characters
with a note when something was left out. A ref belongs to the page that is open: a navigation or a new
snapshot replaces them, and an old ref answers *The page has no element 7; take a new snapshot.*

The refs are also what the other tools take: `click`, `type`, `press` (an optional ref to focus),
`hover`, `drag` (one ref to pick up, one to drop onto), `scroll` (an optional ref to scroll instead
of the page), `select_option` and `set_checked`.

Only elements a person could see are numbered. A page that paints content but hides it from screen
readers with `aria-hidden`, or makes it transparent, is read in a second **drawn** pass when the first
pass finds no text and no controls; a visible form whose controls are named `hidden` or `inert` still
counts, because the host reads those attributes from the prototype and a same-named form field cannot
shadow them (OpenBot 0.69–0.70). If both passes are empty, a PNG of the tab is captured and attached
to the reply as a file card, so the agent gets a picture instead of an empty tree.

## What each tool does

| Tool | What it does |
| --- | --- |
| `navigate` | Opens an http(s) address, or goes `back`, `forward` or `reload`, waits for the page to finish loading by polling `document.readyState`, and returns a snapshot. Reload bypasses the cache (0.70). A bare host gets `https://`. Other schemes are refused. |
| `snapshot` | The tree above. Use it first, and again after anything that changes the page. An empty page gets a screenshot attached instead. |
| `screenshot` | Saves a PNG of the bot's tab (`Page.captureScreenshot`) and attaches it to the reply as a file card, named from the page's host and the time (OpenBot 0.68). |
| `click` | Real mouse input at the element's centre (`Input.dispatchMouseEvent`), with an optional button, double click and held modifiers. |
| `drag` | Presses the element at `from`, moves with the button held to `to`, then releases (OpenBot 0.70). Both ends must be far enough apart and inside the page, or no input is sent. |
| `type` | Focuses the field by ref (or uses what the page has focused), then `Input.insertText`. `mode: replace` (the default) replaces the text, `append` adds to the end, and `submit` presses Enter afterwards. |
| `press` | `Input.dispatchKeyEvent` for one key or shortcut: `Enter`, `Tab`, `Escape`, arrows, `F1`–`F12`, one character, or `Control+A`/`Meta+Shift+P`. |
| `hover` | Moves the mouse onto the element, for menus and tooltips. |
| `scroll` | A mouse wheel turn of `deltaX`/`deltaY` pixels on the page or an element. |
| `select_option` | Selects native `<select>` options by value or label and dispatches `input` and `change`, so a framework sees it. |
| `set_checked` | Checks or unchecks a checkbox or radio button as a click would, with the events a page listens for. |
| `wait_for` | Waits for a text, an address part, `load`/`domcontentloaded`, or `dom-quiet` (the page changed nothing for half a second), then returns a snapshot. |
| `evaluate` | Evaluates JavaScript in the page's own context, with its DOM and cookies, and returns a JSON value up to 64000 characters. For reading what the tools cannot, or a control they cannot reach. A value that cannot be JSON-encoded is reported as such. |
| `run` | A list of the steps above (plus `sleep`) in one call, at most 200 steps and 25 s, returning a final snapshot. A failed step stops the run; the steps before it stay done. A step list, not JavaScript. |

Actions answer with a short status (`{"clicked": "3 button \"Sign in\" <button>"}`, `{"typed": 12,
"into": …}`); add `snapshot: true` to any of them to get the page again in the same call. A
`screenshot` answers with the published file's name and size; the PNG itself shows in the chat as a
file card. A snapshot that finds nothing to read answers with the same card and a note.

## Limits and failures

| What | Limit |
| --- | --- |
| One action's wait | 15 s, 20 s if `timeoutMs` asks for more |
| One `run` | 25 s, 200 steps, `sleep` at most 25 s per step |
| Text typed | 20000 characters |
| JavaScript evaluated, and returned | 64000 characters |
| Address | 2048 characters |
| Snapshot | 400 elements, 40000 characters |
| Screenshot | 100 MiB, the same limit as any shared file |
| Drag | 100000 pixels from the page's origin, at least 8 pixels between the two ends |
| Scroll | 100000 pixels either way |

The waits stay under the 30 s a chat tool call has in `host/src/mcp.rs`, so an agent gets the page's
answer rather than a dead call. Failures say what was missing and what to do next: a lost ref asks for
a new snapshot, an element that cannot take typing says so, a page that never showed the text says
*The page did not show "Invoices" within 15 seconds; take a snapshot to see where it is.*, and a run
that fails names the step that failed and says the steps before it stay done.

A page renderer that dies is named too (OpenBot 0.78.0). When a renderer-side CDP call fails, the host checks the page's Inspector agent: `Inspector.targetCrashed` is Chromium's event for a dead renderer, but the inherited pipe forwards replies rather than events, so `Inspector.enable` (bounded, followed by a browser-level target check) is the signal the tool can read. The result then says *The page crashed (Inspector.targetCrashed). Reload it with navigate direction=reload, then take a new snapshot.* instead of a generic refusal or a snapshot from before the crash. The pure `crash_notice` decision in `host/src/screen/browser_tools/decisions.rs` picks one fixed sentence from the CDP label; no address, page text or crash reason is logged or sent to analytics.

A website `alert`, `confirm` or `prompt` no longer covers the window until somebody clicks OK (OpenBot triage-15938). The bot's tab enables the Page domain, the inherited pipe forwards the `Page.javascriptDialogOpening` event instead of dropping it, and the host sends `Page.handleJavaScriptDialog` with `accept: true` as soon as it arrives, so the page script can carry on. The dialog's kind and message come back in that tool call's `dialogs` field (an array of `{kind, text, truncated}`), so the agent reads the text as data; the pure `dialog_notice` decision in `host/src/screen/browser_tools/decisions.rs` bounds it to 2000 characters, collapses whitespace and gives it a fixed label (`alert`, `confirm`, `prompt` or `dialog`). Only the tool name reaches analytics, never the message. A page still at a sign-in address or showing a visible password field is excluded from auto-accept, so its existing approval-card path stays exactly as it was; a `beforeunload` dialog is left for a person too, and the person's own protected browser never auto-accepts one.

Closing a browser is bounded too (OpenBot 0.74.0): a close waits at most ten seconds for an earlier
action that a stalled page holds, then closes the browser under it, so the tool call ends and the
agent's turn is not left on "working" until someone presses Stop. A script the agent ran with
`evaluate` can still live in another tab of the site, including a popup it never scripted, so
`evaluate` returns a note telling the agent to close or navigate the other tabs of that site, to ask
the user to close tabs it cannot see, and then to reload the page with `navigate direction=reload`
before retrying. Codync's browser tools have no saved-logins or card-fill surface; the note is the
guidance that a secure fill would need, and a secret would go only to the page, never to the agent.

## Cookie banners, bot checks and site memories

The bot's tab refuses cookie banners on its own (OpenBot `browser.md`). An isolated-world script
installed only on that tab (`worldName: "codync-consent"`, via
`Page.addScriptToEvaluateOnNewDocument`) looks for about 15 seconds for a visible **Reject all** or
**Only necessary** control, including one inside a shadow root, and clicks it once. The pure
`consent_button(node, text)` decision in `browser_tools/decisions.rs` is mirrored in `page.js`: a
hidden, disabled or non-interactive control never counts, an "accept", "allow all" or "agree" choice
is never a refusal even when the banner has one, and only a fixed refusal label leaves the page. The
host it refused is remembered in that browser profile, so the next visit keeps the refusal. A person
watching or taking over the tab keeps their banner: the script is removed while they are there and
put back when they leave. The person's protected browser and sign-in cards use other pipes and are
never touched.

A page that puts up a bot check is reported in the snapshot (OpenBot `browser.md`). The page-side
`challenge()` op gathers only bounded markers — visible words plus the class, id and site key of
elements named for a check — and the pure `challenge_status(html, clicked)` decision turns them into
one fixed `challenge` field (`status` and `next`): `none` for an ordinary page, `checkbox` with the
one allowed ordinary click for Cloudflare/DataDome/Turnstile/reCAPTCHA/hCaptcha checkbox checks, and
`takeover` for a puzzle or for a check still standing after that click. The next step tells the agent
to click the checkbox once and re-snapshot, or to ask a person to take over; no puzzle is solved and
no page text from this decision reaches analytics. A navigation starts a fresh allowance for the
click. The field is also printed as one bounded line at the end of the snapshot text, so every path
that ends in a snapshot (including `navigate`, `wait_for` and `run`) shows it.

A lasting preference the user states for one site can be saved as a `MEMORY.md` line beginning
`@site example.com ` (OpenBot `browser.md`). A snapshot shows up to five matching site memories for
the page's host and its parent domains as `siteMemories`, once per host per turn. The pure
`site_memory_match(site, host)` decision in `chat/memory/site.rs` requires a dot boundary, so a hint
for `example.com` also applies to `shop.example.com` but not to `notexample.com`. They reuse the
memory files and store the bot already has, are shown as data (not instructions), and cannot be
created by a page or a tool result — only by the user's own request.

## Sign-in and takeover cards (OpenBot 0.77.0)

A saved password that lands on a slow page no longer hands the browser to the user (OpenBot nightly). After
a `type` with `submit`, the host gives the site a three-second grace to leave the login form: the
`sign_in_complete` decision in `host/src/screen/browser_cards.rs` checks the address before and
after the Enter press and whether a visible password field is still there. A site that moves on —
even slowly — gets a snapshot of the signed-in page and the agent continues; a site that never moves
and stays on the form keeps the existing handoff path exactly as it was. Only a submitting `type`
waits, and the wait stays inside one tool call's deadline. No password or payment approval card is
skipped: those are explicit, separate handoffs that this path never touches.

The browser view and the card code share pure decisions in `host/src/screen/browser_cards.rs`, with
the client-side copy decisions in `apps/desktop/src/renderer/lib/browser-cards.ts`. Every message is
a fixed label; an address is read only to classify a page and is never logged or sent to analytics.

- A browser that comes back from a sign-in page whose request has ended says *This sign-in is no
  longer active. Go back to Codync and start a new sign-in.* instead of a dead page. The host answers
  `browserPanelNotice` with the words; the desk only renders them.
- A page whose script never ends is bounded. A tool call that gets no CDP answer after 20 seconds
  reports *This page is not answering; a script may still be running. Reload it to continue.* and
  names the fix. Navigation (`navigate` in either direction, reload, a new address) stops the stuck
  page first (`Page.stopLoading`) before loading the next one, in the bot's browser and in the
  Browser panel. The desk shows the same notice with a **Reload this page** action
  (`browserPanelReload`).
- When a site refuses a server sign-in, the card keeps one action, **Sign in on this computer**, and
  never opens the server's browser preview. In a browser it drops Open and says **Sign in from the
  desktop app**. The host method `browserPanelSignInOpen` closes the window and says *The sign-in
  page could not load. Check the internet connection and open the sign-in window again.* if that
  page fails instead of leaving a blank window.
- A takeover or ask-help card stays open and answerable after the agent's wait ends; the answer goes
  to the agent as the person's next message. Only the card's own deadline or an answer closes it.
  The island says that Open shows the page in the main window, where it can be typed in.
- An app sign-in started on a phone or another computer can be finished here: paste the address that
  device's browser shows at the end. The client parses only `state`, `code` and `error` and passes
  them to the existing connector OAuth finish path (`connectorSignInFinish`); no other part of the
  pasted address leaves the computer.
- A page an agent shows keeps its tab across new messages in a long chat. `browserPanelOpenPage`
  activates the remembered tab by page key rather than loading the address again, so clicks and
  selections on it survive; the map is bounded and forgets tabs that are closed.

## Privacy

The browser is the bot's own: an isolated profile and context, no cookie of the owner's browser, and
it is removed when the browser stops. Tool results are the page's own text and structure, which the
agent already reads to work, plus whatever `evaluate` returns. A screenshot is an immutable copy in
that bot's own file store, staged for the share and removed from staging once it is published; only
the PNG goes to the chat. No secret is ever passed to these tools: the vault, saved logins and card
fills are not part of this surface (see
[OpenBot's browser tool suite on Codync](../plans/openbot-browser-suite.md#not-ported-and-why)).
