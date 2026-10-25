# OpenBot experience integration

Reviewed 2026-10-25 against Codync `eded5f5f` plus the local 2.26.0
fallback/pairing changes. This is a source investigation and implementation
specification, not a claim that these additions ship in the installed apps.

## Reference checkouts

- `C:/Users/Edwin Villeda/Documents/Openstark`: package name `openbot`,
  package version `0.47.1`.
- `C:/Users/Edwin Villeda/Documents/openstark-0.47.1`: package name
  `openstark`, package version `0.48.2`. The directory name is not its version.
- Both are reference implementations. Their source is under PolyForm
  Noncommercial 1.0.0. Build Codync's behavior using its existing components
  and host APIs; do not import their implementation or catalog as a dependency.

## Grok: runtime, account and model are separate

OpenBot does not demonstrate a second Grok provider independent of the Grok CLI.
It resolves a `grok` executable from system candidates or an application-managed
runtime, checks the minimum version, and runs `--no-auto-update agent stdio`.
The managed runtime uses pinned distribution artifacts and SHA-256 digests.
On Windows it also checks `.grok/bin/grok.exe` and the WinGet links directory.

Its driver supports browser login and device authentication, account isolation
through `GROK_HOME`, and ACP authentication advertised as `cached_token` or
`xai.api_key`. It queries `_x.ai/auth/info` and `_x.ai/billing`. Availability of
a process, a signed-in account, an advertised model and a usable quota are
different states; none should be inferred from an icon or a model's name.

Codync already has backend ID `grok`, executable `grok`, registry entry
`grok-build`, and device-code login. Its visible name is `Grok Build`. In the
installed host's hello, Grok was unavailable/uninstalled, while Antigravity was
available/installed. The host version was 2.26.0. No Grok login was attempted.

Implementation requirements:

- Keep the stable backend ID and existing bot settings. A display-name change
  must not create duplicate backends or pretend to add another engine.
- Show runtime installation, account sign-in, model availability and failures
  separately, with an actionable connection path in the model picker.
- Query capabilities/models from the actual agent instead of cloning the model
  names in the supplied screenshots.
- If the goal is Grok without any CLI, specify an xAI API adapter separately:
  that requires API credentials, tool execution, streaming and permission
  integration. It is not what the inspected OpenBot driver implements.

Evidence in each reference checkout: `src/backend/cli.ts`,
`src/backend/grok-client.ts`, `src/backend/provider-drivers.ts`,
`src/main/provider-runtime-descriptors.ts`, `native-runtime.lock.json`.
Codync: `host/src/agent/backends.rs`.
Official protocol: https://docs.x.ai/build/cli/headless-scripting

## Usage and the island

Codync's host currently polls Claude and Codex. Its general clients render
provider reports, but the island's Usage view explicitly takes only the first
two providers and the first three windows. Adding a provider at the host alone
does not provide a usable multi-provider island. The host returned no usage
providers during the read-only installed-app check; missing data is not 0%.

The 0.48.2 reference adds Antigravity quota collection using a private `/usage`
request. It guards this with an ACP-server version check: 1.4.1 or later handles
the command locally; 1.3.0 would send it to the model. Never send a speculative
`/usage` prompt to an unverified adapter. API-key authentication may have no
subscription quota. Its parser also distinguishes Gemini model quotas from
Claude/GPT-through-Antigravity quotas.

Build these separate measurements:

- Subscription limits: provider, account, model group, window, used/remaining
  percentage, reset instant, source and last observation.
- Task activity: bot, current action, elapsed time, waiting/working/completed/
  failed state; use host state, not a percentage guessed from elapsed time.
- Token usage and estimated API cost: record actual counters by turn/model,
  handle cumulative counters without double-counting, and label estimates.
  A subscription quota is not a token total or a billed dollar amount.
- Context occupancy: show separately from plan quota and task completion.

Preserve unknown values; display unavailable/unsupported/stale states. Refresh
must be bounded, deduplicated and scoped to the right account. Never combine
two accounts into one percentage. Provider credentials remain in the host.

Presentation:

- Desktop: a Usage view with provider/account selection and expandable windows.
- Island: compact selected-provider meter; an accessible provider selector or
  paging for all providers, keeping the existing small window geometry.
- Android/iOS: the same host report and source/age/account semantics, adapted
  to phone controls. TUI: selectable accounts/windows and readable unknowns.
- The user clarified that "web" means the desktop application. Keep these
  changes in the existing Electron renderer; no browser dashboard or public
  website changes are required.

Evidence: Codync `host/src/usage.rs`,
`apps/desktop/src/renderer/views/island/IslandUsage.tsx`,
`apps/desktop/src/renderer/views/usage/usage-model.ts`,
`apps/android/app/src/main/kotlin/com/codync/android/UsageScreen.kt`,
`host/src/tui/view/usage.rs`; reference 0.48.2
`src/backend/antigravity-usage.ts`, `src/backend/cli.ts`,
`src/backend/provider-drivers.ts`; both references
`src/backend/agent/usage-collection.ts` and
`src/backend/database/agent-usage.ts`.

## Routines, scheduled tasks and runs

Codync already owns routine compilation, validation, time zones, preview,
execution, retry, once/interval/cron schedules and webhook/watch/event triggers.
Do not install a second scheduler in a renderer or phone app.

OpenBot adds useful organization: a host-wide calendar merges routines and
their historical runs, then projects future occurrences in a bounded date
range. The UI separates scheduled occurrences from actual run outcomes.
Its marketplace can also install routines as part of a prepared agent.

Adaptation:

- A global Tasks entry with Upcoming, Running, Needs attention and History.
- Calendar and list views backed by host-computed occurrences. Android uses
  a compact agenda; TUI uses the equivalent chronological list.
- Reuse the existing per-bot editor and test-run controls. Opening a task
  identifies its bot, schedule/time zone, next run and most recent outcome.
- Keep one-off schedules, recurring routines and execution records distinct.
- Preserve advanced/unknown triggers rather than replacing them on save.
- Define missed-run/restart policy explicitly and prevent duplicate execution.

Evidence: references `src/backend/agent/routine-scheduler.ts`,
`packages/team-client/src/routine-calendar.ts` and `routine-schedule.ts`;
Codync `host/src/routines/`, `apps/desktop/src/renderer/views/routines/`,
`apps/android/app/src/main/kotlin/com/codync/android/RoutinesScreen.kt`.

## Marketplace and simple bot creation

The screenshot includes different things: provider runtimes in the model
picker, ready-made agent profiles, and installable skills. They should remain
distinct in Codync. Its current Extensions page primarily lists agent runtimes,
connectors and skills; that is not a catalog of ready-made bot profiles.

OpenBot's agent installer creates a profile and can install versioned skills
and routines, with source attribution and partial-failure handling. Its remote
catalog requires its own service/authentication. Reusing the design does not
give Codync permission or credentials to use that service or its install counts.

Adaptation:

- Marketplace sections: Bots, Agents/providers, Connectors and Skills.
- Start with a Codync-owned local catalog of ready-made bot profiles; use real
  descriptions and previews. No invented download counts or remote catalog.
- New bot: name, purpose, avatar and available model; expose account/connection
  state at selection time and keep advanced configuration accessible.
- Profile detail: instructions, skills, routine definitions and required
  permissions before creation. No account secrets in a profile package.
- Creation stays host-owned. Use stable creation IDs, verify packages, handle
  partial installation, and require explicit access grants.
- Do not silently enable scheduled routines, shell access or connectors merely
  because a profile was selected.
- Equivalent creation and inspection on Android, iOS, desktop and TUI.

Evidence: references `src/main/agent-marketplace-service.ts`,
`packages/team-client/src/marketplace-catalog.ts`,
`packages/ui/src/features/marketplace/`; Codync
`apps/desktop/src/renderer/views/marketplace/MarketplaceView.tsx`,
`host/src/api/marketplace.rs`, `apps/ios/Kit/Sources/CodyncUI/Bots/NewBotView.swift`.

## Delivery order and acceptance

1. Provider/account/model states and usage ingestion, starting with verified
   Antigravity support; remove the island's two-provider omission through
   selection, not unbounded growth. Test unsupported adapter versions, absent
   quota, all windows, account isolation, stale data and reconnects.
2. Simple profile creation and a Codync-owned bot catalog using existing host
   creation APIs. Verify retries, unavailable models and partial installs.
3. Global task agenda/calendar over the existing scheduler. Test time zones,
   missed occurrences, restart, cancellation and execution-result attribution.
4. Validate the same reports in the installed desktop, island and Android app;
   the user's "web" target is the desktop app, not a new browser client.

At this investigation checkpoint, no new provider was connected, no API key
was read, no external routine was enabled and the installed 2.26.0 apps were
not replaced. Existing fallback/QR source changes remain uncommitted. Physical
acceptance is still required for subsequent implementation on Windows/Android;
unavailable Apple/Linux environments must be reported explicitly.
