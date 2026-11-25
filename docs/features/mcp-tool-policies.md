# MCP policies and execution receipts

The host owns tool policies for remote HTTP/SSE, local stdio commands and
connected Composio apps. Windows, Android and TUI expose **Tools & requests**
on installed connections. Apple implementations are excluded by instruction.
This boundary does not sandbox independent tools/network access in an ACP harness.

## Exact approvals and defaults

An explicit owner `allow`, `ask` or `deny` overrides the default. Otherwise,
`readOnlyHint: true` observed in actual server metadata defaults to Allow;
missing/false annotations default to Ask first. Caller arguments cannot supply
those defaults. An annotation is a server trust choice, not proof of no effects;
the owner may override it. Reset restores the observed server default.

The host saves exact arguments, tool, bot, connector and a one-hour expiration.
Limits are 128 KiB per request and 100 unexpired pending/approved requests per bot.
An approval accepts only request ID and decision. The original waiting proxy
claims the saved message atomically once and rechecks live authorization before
dispatch. Declined, expired and revoked requests cannot dispatch.

Proxy identity binds command, arguments, environment, target, headers and stable
OAuth configuration. Token refresh and cosmetic edits do not invalidate it.
Runtime replacement, removal and bot disablement revoke earlier authorizations;
replacement removes inherited catalog/policy state. Checks and claims share the
configuration lock.

Composio approves the actual provider tool slug and arguments, bound to account,
key and user. Dispatch rechecks that binding and reserves a second durable claim
before calling the provider. Repeating the dispatch API cannot repeat execution.
Missing/ambiguous success fields remain unknown outcomes. Credential/account
replacement revokes inherited policy state.

## Test connection and the chat's tool latch

A server that failed to start in a chat had its tools left out of that chat's
agent session until a new chat. A passing **Test connection** (`connectorVerify`)
now clears that startup-failure latch: the host refreshes the tools of every bot
using the connector, so the next message declares them again. This applies both
to servers that need sign-in and to servers with no sign-in. A check that still
fails keeps the latch and its recorded failure message.

The latch only restores tool declarations. It never changes an `allow`, `ask` or
`deny` policy: a tool denied by policy stays denied at authorization time. The
client label for the success state is "Tools available from the next message"
(English source).

A server that failed to start also gets one plain notice in the bot's chat, on
every provider, not just Pi and custom endpoints (plan 003). The notice names the
server only — no error text or tool output leaves the host — and repeats only
when its classified cause changes (`auth`, `unreachable`, `tool`): the same cause
seen twice stays quiet. The host keeps at most 64 causes per bot for this dedup.

A saved local server runs in each bot's working directory, so a relative path in
its command only resolves where that file exists. The desktop MCP form and the
tool-policy panel both show that limit (the panel's label is "Working directory").

## Management and read-only recovery

The sheet contains overrides, an exact-argument request inbox, confirmed bulk
revocation and saved-result lookup by request ID. TUI: Ctrl+T opens a connection,
Tab switches tabs, Enter inspects/edits, `n` adds, Delete resets, `l` loads more,
Ctrl+X confirms revocation, and `g` opens receipt lookup. Receipts are read-only.

Actual JSON-RPC results/errors are saved as immutable receipts, up to 512 KiB.
The first result wins; a different replacement is refused. Receipts survive
restart and can be read via `getMcpToolResult {connectorId,id}` with Control
authorization, even after expiry/removal. Missing means **unknown**, not failure
or permission to execute again. `dispatched` is only a reservation. Receipts may
contain sensitive tool output and are stored like transcripts.

Owner APIs: `getMcpToolPolicy`, `setMcpToolPolicy`, `getMcpToolSettings`,
`resetMcpToolPolicy`, `listMcpToolCalls`, `getMcpToolCall`, `resolveToolCall`,
`revokeMcpToolCalls`, `getMcpToolResult`. Proxy preparation/claim/completion and
Composio dispatch methods require the local caller.

## Cancellation and validation

Readers observe cancellation and EOF during approval; overload fails closed.
Local processes use owned process trees/Windows Job Objects; revocation sends
MCP cancellation to the server. Remote transport follows no redirects, and
legacy SSE requires the same origin. Bounded queues drain actual replies before
shutdown where possible. A server may already have produced effects: cancellation
is cooperative and cannot guarantee rollback. Unknown outcomes and bridge
restarts never trigger automatic execution retries.

Fourteen interpreted Node/SQLite tests execute production schema/SQL for policy
transitions, management, defaults, receipts and provider execution claims. Eleven
tests exercise the actual Windows management model. Rust changes/regressions were
source-reviewed, not compiled or executed. Native and live provider checks remain
pending. No application was compiled or installed.
