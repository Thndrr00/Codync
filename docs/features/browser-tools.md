# Browser tools

A bot with **Use the computer** on and its browser permission gets twelve browser tools from the
built-in `chat` MCP server (`host/src/mcp.rs`): `navigate`, `snapshot`, `click`, `type`, `press`,
`hover`, `scroll`, `select_option`, `set_checked`, `wait_for`, `evaluate` and `run`. They drive a
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
`hover`, `scroll` (an optional ref to scroll instead of the page), `select_option` and `set_checked`.

## What each tool does

| Tool | What it does |
| --- | --- |
| `navigate` | Opens an http(s) address, or goes `back`, `forward` or `reload`, waits for the page to finish loading by polling `document.readyState`, and returns a snapshot. A bare host gets `https://`. Other schemes are refused. |
| `snapshot` | The tree above. Use it first, and again after anything that changes the page. |
| `click` | Real mouse input at the element's centre (`Input.dispatchMouseEvent`), with an optional button, double click and held modifiers. |
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
"into": …}`); add `snapshot: true` to any of them to get the page again in the same call.

## Limits and failures

| What | Limit |
| --- | --- |
| One action's wait | 15 s, 20 s if `timeoutMs` asks for more |
| One `run` | 25 s, 200 steps, `sleep` at most 25 s per step |
| Text typed | 20000 characters |
| JavaScript evaluated, and returned | 64000 characters |
| Address | 2048 characters |
| Snapshot | 400 elements, 40000 characters |
| Scroll | 100000 pixels either way |

The waits stay under the 30 s a chat tool call has in `host/src/mcp.rs`, so an agent gets the page's
answer rather than a dead call. Failures say what was missing and what to do next: a lost ref asks for
a new snapshot, an element that cannot take typing says so, a page that never showed the text says
*The page did not show "Invoices" within 15 seconds; take a snapshot to see where it is.*, and a run
that fails names the step that failed and says the steps before it stay done.

Closing a browser is bounded too (OpenBot 0.74.0): a close waits at most ten seconds for an earlier
action that a stalled page holds, then closes the browser under it, so the tool call ends and the
agent's turn is not left on "working" until someone presses Stop. A script the agent ran with
`evaluate` can still live in another tab of the site, including a popup it never scripted, so
`evaluate` returns a note telling the agent to close or navigate the other tabs of that site, to ask
the user to close tabs it cannot see, and then to reload the page with `navigate direction=reload`
before retrying. Codync's browser tools have no saved-logins or card-fill surface; the note is the
guidance that a secure fill would need, and a secret would go only to the page, never to the agent.

## Privacy

The browser is the bot's own: an isolated profile and context, no cookie of the owner's browser, and
it is removed when the browser stops. Tool results are the page's own text and structure, which the
agent already reads to work, plus whatever `evaluate` returns. No secret is ever passed to these
tools: the vault, saved logins and card fills are not part of this surface (see
[OpenBot's browser tool suite on Codync](../plans/openbot-browser-suite.md#not-ported-and-why)).
