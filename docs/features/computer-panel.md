# Computer panel and capability control

The host owns the panel, workspace operations and capability checks. Windows and
TUI expose Status, Files, Terminal and Activity. Android exposes Status, Files
and Activity, following catalog item 26's explicit mobile adaptation. It can
revoke shell permission while a command runs elsewhere.

`computerPermissions` contains independent `browser`, `files` and `shell` booleans,
all false when absent. Existing bots must explicitly enable capabilities after
upgrading. `computer` remains the master switch. Browser means the existing
desktop driver (apps, mouse, keyboard), not an isolated web browser. These gates
cover Codync's built-in tools; an ACP harness's own filesystem, network, terminal
or computer tools remain governed by its independent permissions.

The panel APIs use Control authorization: `computerPanel`, `listComputerFiles`,
`readComputerFile`, `runComputerCommand`. Agent-facing `workspace_list`,
`workspace_read` and `workspace_shell` use the same host implementation and gates.
Only metadata enters activity history; actor identity distinguishes bot, local
owner and paired device. Existing activity records retain their bot identity.

Files are relative to the bot workspace; parent traversal and canonical targets
outside it are refused. Listings stop at 500 entries and previews at 128 KiB of
UTF-8 text. Link rows cannot be opened from the panel. This is a workspace
navigation policy on the real host, not a filesystem sandbox against concurrent
local processes changing paths.

Terminal runs one noninteractive command at a time per displayed client panel,
in the workspace, with a 30-second deadline and 64 KiB per output stream. Windows
starts the process suspended, assigns the existing kill-on-close Job Object,
then resumes it. Revocation/timeout drops that ownership and stops descendants.
No command is retried automatically. A lost response has an unknown outcome;
inspect before manually retrying. Closing a panel does not revoke permissions;
a command can continue until completion, revocation or its deadline.

Revocation invalidates capability leases, including work already awaiting I/O.
Desktop transport rechecks the lease immediately before dispatch. On Windows,
revocation also terminates the owned external driver and its Job Object process
tree, including an action already dispatched. Effects completed before termination
cannot be rolled back. This native process path remains source-reviewed only.

## Manual password/code entry

`beginProtectedEntry {botId,site}` accepts an HTTPS site without credentials and
stores only its origin. It takes user control, invalidates captures, pauses all
Codync computer tools (including screenshots, files and shell), invalidates
existing leases and asks active bot harnesses to stop. The owner types directly
on the real host after checking the website. No password/code value is accepted
by this API or returned to an agent. `finishProtectedEntry {id}` requires the
current entry identity, releases control and invalidates captures again. Ordinary
handback cannot bypass an active entry. This state is intentionally held until
explicit completion; it never auto-resumes a bot during secret entry.
Public metadata is persisted before activation and restored before actors start
after a host restart. An unreadable record refuses startup. Finishing removes
the durable record before releasing the in-memory gate.

Harness stop remains cooperative; independent harness tools are not isolated.

## Protected autofill on Windows

The local Windows owner can open a dedicated private Chrome/Edge browser with
`openProtectedBrowser`, then fill a saved login or a manually entered code using
`fillProtectedLogin` or `fillProtectedCode`. These APIs are local-only and are not
agent tools. Existing Node.js and Chrome/Edge installations are required; this
flow installs nothing. Android can control the manual pause, but cannot send
autofill secrets to the local-only APIs. TUI offers the local owner the same flow.

The host owns the browser process tree and communicates over inherited anonymous
pipes, without a TCP debugger. Before resolving a saved password, it checks the
exact approved HTTPS origin and focused input node. The same node is checked
again at insertion. Passwords require a password input; codes require an explicit
one-time-code field. Values never enter clipboard, agent history, command-line
arguments or activity logs. Interrupted transport is poisoned and cannot be
reused for another fill. An unknown outcome is never retried automatically.
This supports the owned browser, not arbitrary existing tabs or native apps.

Finishing closes the owned browser by default. Explicit `keepSession` first
destroys its previous pages and opens a clean page in the same private context.
Any unverified surviving target closes the entire browser before tools resume.
Only a verified reset preserves session cookies. The approved website receives
the value entered into its form; this is not isolation from that website.

## Validation without compilation

Eleven interpreted Node tests exercise the real Windows panel model: host intents,
stale permission loads, out-of-order browsing, revocation, duplicate command
clicks, preserved errors, manual-entry payloads and remounting. Three Node/SQLite
tests exercise production activity queries and schema, including legacy actors,
retention cleanup and deleted bots. Rust capability regression tests were added
but were not run because that would compile code. Source reviews corrected the
deletion/lease race, polling during permission mutations, stale refreshes and
TUI path/tab state. Windows, Android and TUI also show research/Slack status;
unavailable or locked credentials remain explicit. Five tests execute the actual
protected-field JavaScript, including exact origin, focus and numeric-code refusal.
`node tools/verify-protected-browser.mjs` exercised the production JavaScript and
pipe broker in installed Chrome with dummy values. It verified protected filling,
page destruction and the whole-browser-close fallback for unverified targets.
It did not run the Rust host or the native application integration.

No Rust/Android compilation, TypeScript type checking, packaging or real-app e2e
has been performed. Apple-specific implementations are excluded by instruction.
