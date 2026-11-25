# Provider plan usage

The host owns quota inspection; desktop, Android, iOS and TUI render its `usage`
report. Percentages describe a provider's plan window, not task completion,
context occupancy, token totals or a billed amount.

Claude asks its ACP adapter's own `/usage` report first (the account that
session is signed in to), then the CLI's `/usage` when the adapter cannot
answer; the status-line ingest and the adapter's rate-limit event still update
the same windows. The CLI probe obeys the bot's "Use my Claude Code setup" switch (OpenBot 0.70.0):
when no Claude bot keeps its own setup on, it runs from the host's probe folder rather than the home
folder, so a slow user setup can't hold the check to its deadline; with the switch on the old
behavior stays. The model-list check already probes from that folder. Codex asks its app-server
(`account/rateLimits/read`) and falls back to the newest local rollout's
`rate_limits` when the app-server cannot answer. Grok starts its CLI's ACP
server and calls `_x.ai/billing`; the current period's share used becomes the
window (weekly or monthly), and a period with no recorded usage is zero, not
unknown. Antigravity inspection starts an isolated ACP session without MCP
servers or filesystem/terminal capabilities. It rejects every client tool
request, caps response text at 64 KiB and times out each request after 20
seconds. `/usage` is sent only when the adapter's initialization advertises a
stable version of at least 1.4.1. Unsupported or unknown versions receive no
prompt. The inspection process is stopped on exit. OpenCode asks the console's
own `GET /zen/go/v1/usage` for the credential the CLI is signed in with
(OpenCode 2's active credential, `auth.json`'s `opencode-go` key, or
`OPENCODE_API_KEY`); a signed-out CLI has no key and no row. Only backends that
are installed and not explicitly signed out contribute windows, so a local
history file cannot become a bar.

The Antigravity report is shown as **Gemini** (the Google plan it reads); an
isolated account uses provider ID `antigravity:<account ID>` and its saved
display name. Reports and model groups remain separate. Percent remaining
becomes percent used. A missing reset stays unknown; valid UTC resets become
epoch milliseconds. API-key accounts may have no plan quota. Invalid/missing
quota never becomes a fabricated zero.

`status` is an optional additive field: `unsupported` asks to update the adapter;
`unavailable` means the current account could not report a quota. These reports
have no numeric windows. A signed-in agent that reports nothing still gets a
placeholder (`unavailable`) so its counters don't vanish. Older clients ignore
the field. A failed observation replaces the old windows rather than presenting
them as fresh. Deleted profiles are removed from the report on refresh.
Concurrent refresh requests wait for the active refresh and then return its
result, so a manual refresh never answers with a half-filled list; the existing
five-minute polling interval is retained. The newest report is saved in
`usage.cache`, so a restarted host shows the last reading while the first probe
runs.

The island pages through two providers/accounts at a time and scrolls windows
inside each card. General desktop, phone and terminal usage views already list
all reports. Their unavailable states and Antigravity source labels match; the
island's pagination is desktop-only because the other clients do not have its
small fixed window. The user clarified that "web" means the Electron desktop,
so the public website is unaffected.

## Starting a provider again, and custom endpoints

A provider that would not start several times in a row gets a `NotStarting`
rest, so all bots skip it while it recovers (OpenBot 0.74.0). A message or a
routine is a new turn: `agent::fallback::start_again` clears only that
start-failure latch, so the next run tries the agent once. It does not retry
inside the failing loop, and it leaves usage, rate-limit and signed-out rests
alone. A custom OpenAI-compatible endpoint at `https://api.openai.com/v1` with
a key uses the Responses request rather than `/chat/completions`: newer models
such as GPT-6 Astra refuse function tools together with `reasoning_effort` on
the compatible endpoint. Both the tools and the effort stay in the request; a
keyless or other endpoint keeps the compatible adapter, and the key is only a
header, never part of the body or a log.

Codync's `custom` backend normally runs the command the user saved, so the HTTP
path is **opt-in per bot**: `setCustomProviderMode` stores `openai-compatible`
on that bot (the old `cli` default keeps every existing setup unchanged), and
`customProviderMode` reads it back. Only a bot whose saved `OPENAI_BASE_URL` is
exactly `https://api.openai.com/v1` **with** a saved `OPENAI_API_KEY` takes the
HTTP path; the host posts one bounded, egress-checked request built by
`agent::config::custom_request` to `/v1/responses`, with the key only in the
`Authorization` header. Every other case — mode off, a keyless or different
URL, a keeper-proxied key — leaves the bot's CLI as the launch path. The mode
exists because the OpenAI v1 fix belongs to the request, while a custom bot's
default is its own CLI: switching the transport has to be a deliberate choice,
and analytics reports only the fixed mode name, never a URL or a key.

Source tests cover version gating, invalid/missing quota, conversion and model
groups, distinct account identities, Codex app-server parsing, Grok's proto3
billing periods, the saved reading and Android decoding of unavailable
accounts. Real account/provider and installed-app checks must be recorded
separately; compilation does not establish live quota availability.

## Updating the agent CLIs (OpenBot 0.75.0)

Once an hour the host checks whether a provider CLI it downloaded itself (an ACP
registry binary build) has a newer version, and installs it through the same
registry downloader used to launch an agent — there is only one updater. The
check waits for idle agents, using the same readiness gate as a host update's
restart, for at most ten minutes; an always-busy computer is never held. A CLI
the person installed, a bot pinned to a custom command path and a provider no
bot runs still show **Update available** in Settings › Providers but are never
auto-updated. Once a build is installed the host remembers its version beside
its other update state, so the hourly check does not offer that same build
again until the registry serves a newer one (a changed nightly tag counts as a
new build). A version the person cancels is kept per provider and version for
the life of the host process: that exact build is not started again until the
host restarts, while a newer version is still eligible. **Update all** applies
every eligible provider at once on this computer; there is no joined-server
part because providers belong to a computer.

## Turn trace counts (OpenBot nightly triage-7101)

The host's turn-usage trace reports **Tokens** as only the new tokens — fresh
input, output and cache writes, the same count the daily budget and the Usage tab
spend — and **Cached input** as the cache reads, which each model call repeats
when it rereads the conversation. A short task can no longer show a million-token
total just because the cached conversation was read many times. One pure split in
`usage/tokens.rs` feeds the trace; the provider's own cost report is unchanged,
and a turn from an older host that never reported a cache-read count keeps the
total it had.

## Turned-off models (OpenBot nightly triage-12582)

A model turned off in a provider's Models settings is hidden from an agent's
picker and Backup models list only when no agent still uses it. A model an agent
still answers on (as its own model or a fallback) stays visible for that agent;
a new agent, with no model chosen yet, does not start on it. The host computes
the effective hidden list from the saved bots and the phones apply the same
per-agent rule when they render the picker. On a joined server that server must
also run this version for the host-side decision.

Codync has no OpenBot-style Custom provider settings page with a "Checking"
state or an OpenCode enable button (triage-12259): OpenCode is an agent CLI
listed under Settings › Providers, and custom OpenAI-compatible endpoints are an
opt-in per bot as described above.

## Model lists, accounts and approvals (OpenBot nightly 0.70.0)

`agent::auth::models` caches the last model list for the agents whose catalog can
grow while they stay running (Antigravity / Gemini). The next picker ask after
about ten minutes reads it again, so a model Google grants the account later
appears without a restart; a quiet re-read that fails or comes back empty keeps
the older list. An account added after the first gets a five-second first read
and one full-length retry when that read times out, so a slow new Codex account
still shows its own models. The catalog returned for a backend also carries
`backupModelChoices`: the shared models first, then each account's own models
tagged with that account, so an account-only model can be a backup.

A custom provider's model id with punctuation keeps its vendor form in the bot
and gets an escaped listed form for the agent: `@cf/openai/gpt-oss-120b` becomes
`=40cf/openai/gpt-oss-120b` after the endpoint id. Matching accepts either form,
and hidden models are saved in the listed form, so Cloudflare Workers AI ids
work without changing saved agents that already use plain ids.

A Codex bot at **Ask first** starts with `approval_policy: untrusted` in its
`CODEX_CONFIG`, so it asks before each command and file change, including work
in routines and channels; its reach and network are unchanged. A backend the
person opted into automatic approval keeps `on-request` — that is the "Full
access" reach of an **Always allow** choice. The policy is set when the agent
process starts, so a setting change takes effect from its next message. When a
usage limit stops a bot and every automatic backup is also limited or on hold
after a recent failure, the chat says that models of one account can share a
limit and that a backup on another provider or account lets it continue; the
provider's own text stays in the log. On a joined server the server must also
run this version for these host-side changes.

## Full readings, model picks and recovery (OpenBot 0.78.0)

A plan probe that answers with no windows is a failed reading, not an empty
plan: the provider's last windows and their reset times stay in the report
until the next reading replaces them. A first reading that comes back empty
still adds no provider. The decision is pure in `usage/mod.rs`.

Changing only a model in a bot's settings keeps that bot's provider. A name
that another provider also lists is not a reason to switch; the host applies
the name when the bot's own provider lists it (or no catalog contradicts it),
and otherwise keeps the saved model. The rule is pure in `hub/bots.rs`.

ChatGPT has two failures that do not end the turn where other providers wait.
A capacity error ("at capacity", "capacity limit") is its own fallback cause:
with automatic backups configured and a backup to run on, a turn that already
did work moves straight to the next backup model or account, and the provider
rests only briefly. When the provider the bot was using is no longer
configured (its API address was removed), a Codex chat drops that session,
starts one on the setup saved now and receives the earlier conversation through
the existing handoff prompt, so the chat carries on with a summary. A Codex
settings read that fails with a bare "invalid data" is reported to the person
as "couldn't load your ChatGPT settings; reconnect in Providers and try again";
other errors keep their detail. These rules are pure in
`agent/fallback/{reason,carry,rest}.rs` and tested there.
