# The browser tools on Codync: what was taken

Reviewed 2026-11-18 against Codync `2.47.0`, from OpenBot nightly's `src/backend/browser-tools.ts`,
`browser-host.ts` and the page-side driver they carry. Same rule as [0.70.0](openbot-0.70.0.md):
behavior, action names and wording are copied, and the file that holds a piece names where it came
from.

OpenBot's agents drive an embedded Chromium through a tool per action, and through one `run` tool that
executes a short JavaScript program with the same actions as helpers. Codync's agents had no browser
tools at all: the protected browser (`screen/protected_browser.rs`) exists for the person's sign-ins,
and the panel (`screen/browser_panel.rs`) for browsing, both over a private CDP pipe.

## Landed

| OpenBot item | Codync shape | Files |
| --- | --- | --- |
| A tool per browser action, with `snapshot` numbering the page's elements | `navigate`, `snapshot`, `click`, `type`, `press`, `hover`, `scroll`, `select_option`, `set_checked`, `wait_for`, `evaluate` and `run`, as the chat server's tools for a bot | `host/src/screen/browser_tools.rs`, `browser_tools/tools.rs`, `browser_tools/action.rs` |
| `run`: a short program with the actions as helpers, capped | The same program, with `sleep`, at most 200 steps and 25 seconds; each step is one of the tools, so a failure names the step | `browser_tools/action.rs`, `browser_tools/cdp.rs` |
| The page's half: find an element, describe the page, wait for a condition | `page.js`, injected once per document | `browser_tools/page.js` |
| A snapshot the model can act on | A compact tree with numbered refs, the URL and the title, capped at 400 elements and 40 000 characters | `browser_tools/snapshot.rs` |

The bridge is Codync's own: the tools use the private CDP pipe (`browser_tools/cdp.rs`) on a browser
context and one tab that belong to the bot, instead of OpenBot's Electron views. Every bot reaches
them through the same gate as the computer tools: its `Browser` permission
(`computer::begin(hub, bot, Capability::Browser)`). Off Windows no CDP browser runs, so nothing is
listed there rather than a set of tools that always refuse.

Limits, all explicit: 20 seconds for one action, 25 seconds and 200 steps for a `run`, 20 000
characters typed, 64 000 evaluated, 400 elements and 40 000 characters of snapshot.

## Not ported, and why

- **OpenBot's browser tabs, takeover cards and live view**: Codync's panel and the person's own
  browser already cover them, in their own shape.
- **The upload tool and file pickers**: Codync has no per-tab upload store; a file reaches the page
  through its own change if it is needed.
- **Downloads and dialogs**: they belong to the browser window Codync starts, and the person sees it.

## Checks

`rustfmt --check` is clean on every file of the module, and the pure half (ref parsing, the step cap,
the snapshot formatter, the argument checks) is unit-tested. The module has not been compiled with
cargo on this machine yet; that is the branch's next gate.
