# OpenBot 0.78.0 in Codync

Source: OpenBot nightly tag `v0.78.0` (68 changelog bullets) plus the unreleased fragments on
`origin/main@8e5a791e6`, audited against the 189 commits already integrated on this branch. Same rule
as earlier plans: copy behavior and user-facing text with a short source comment, reuse Codync's own
components and host APIs, and bridge where an OpenBot subsystem does not exist. No
hosted/billing/credits, joined servers, web client, Chrome extension/X, OS packaging or SolidJS.

## State

Four read-only auditors classified every bullet and fragment. Many v0.78 fixes arrived earlier as
unreleased fragments and are already integrated: UTF-8 BOM in `SKILL.md` (`71bf1346f`), MCP Test
connection restoring tools (`25ec11594`), routine questions not closing earlier agent questions
(`0b506b507`), Grok empty probe sessions, CPU capability and Claude sign-in (`faea777ea`), duplicate
agent with links and slow sign-in handoff (`835c7730b`), stale-turn settle for "Stop ends a Gemini
turn" (`182f4087c`), WhatsApp personal-number notice (`91993bf1a`), and per-agent model visibility
(`b44593253`). The remaining portable gaps are grouped into disjoint batches below. Nothing here has
been compiled or run end to end on this machine.

## Already integrated before this tag shipped

| OpenBot 0.78.0 | Codync | Where |
| --- | --- | --- |
| `$` skill picker shows a `SKILL.md` saved with a BOM | Shared readers strip U+FEFF | `host/src/market/skills.rs` |
| Passing Test connection restores MCP tools | Verify clears the failure latch | `host/src/mcp/policy.rs` |
| Routine question doesn't close an earlier agent question | Independent card lifecycles | `host/src/external/question.rs` |
| Grok model checks stop leaving empty sessions | Probe removes only its own session | `host/src/agent/auth/models.rs` |
| Unsupported CPU says the provider can't run here | Capability classification | `host/src/agent/backends/capability.rs` |
| Claude Connect/Add account opens the sign-in page | Safe provider sign-in page | `host/src/agent/backends/sign_in.rs` |
| Duplicate agent keeps linked folders and retries | Workspace copy with bounded retry | `host/src/store/bots/workspace_copy.rs` |
| Slow sign-in after a saved password isn't handed over | Bounded completion grace | `host/src/screen/browser_cards.rs` |
| Stop ends a turn the provider no longer runs | Stale-turn settle | `host/src/agent/bot/turn_rules.rs` |
| New agent starts on a visible model | Hidden models kept only for their agent | `host/src/agent/auth/models.rs` |
| GPT live voice shows what you said | Transcript linked into the chat | `host/src/chat/calls/**` |
| Voice model list shows only transcribable models | classify() realtime/transcribe/speech | `host/src/voice.rs` |
| WhatsApp personal number: no "Working on it…" elsewhere | post_busy_notice gate | `host/src/whatsapp/settings.rs` |

## Portable gaps — batches

### A — steering and turn recovery
- Steer a message sent while the agent starts its turn (`agent/bot/steer.rs`).
- Steer into Antigravity/Devin before the first update.
- Steer again after a Stop that didn't stop the turn.
- A hung tool ends the turn and frees the queue (triage-15551).
- Claude process dies with subagents/background work running: immediate recovery turn (triage-15555).
- Another agent's/routine's message no longer interrupts a busy Claude step (triage-15730).
- Routine "Run in a separate thread": open wait continues in main chat (triage-15775).
- A message sent while a sign-in/password/code card is open doesn't cancel it (triage-15776).
- Gemini shown "working" with no live turn stops blocking the queue (triage-15615).
- OpenCode "the request is invalid" starts a fresh session with history.

### B — fallback, usage and models
- Keep the last Claude plan reading on an empty reading (`usage/mod.rs`).
- Model-only change keeps its own provider when another lists the same name.
- ChatGPT at capacity with Automatic backups moves at once to the next model/account (triage-15582).
- Codex chat continues after a provider's API address is removed, with a summary.
- Codex settings-load failure says so in plain words, not "invalid data".
- OpenCode Ask first asks before commands/file changes (triage-15626).
- OpenCode actions written as text don't count as a completed turn (triage-15547).
- A long Pi answer with lots of code stays complete.
- Pi/custom endpoint says when one of your MCP servers failed to start.

### C — files and cards
- File links with folders open only when folders agree; `report.md:26` opens at the line.
- A card shown during a turn stays where it appeared after being answered.
- The app-connect card goes to your chat when an agent asks a teammate to connect an app.
- Tasks page on desktop is called Threads, with thread wording.

### D — routines, channels and notifications
- Phone push when a workflow reaches a Notification output or a waiting step.
- Channel work sent from a side thread returns to that side thread, not the main chat.
- OpenCode agent creates Codync routines, not its own invisible automations.
- Agent answering a channel contact doesn't promise to pass the message on.

### E — voice and read-aloud
- Read aloud and live voice skip list marks (`-`, `+`, `*`, `•`), task boxes (`[ ]`), quotes and `===`.
- GPT live voice doesn't post the same request/reply twice (triage-15781).
- Read aloud isn't interrupted by the same agent's other work (triage-15567).

### F — browser and screen
- The agent learns when a browser tab crashes (`Inspector.targetCrashed`).
- Screen capture failure gives the macOS Screen Recording guidance (triage-15679).

## Not ported — no Codync equivalent

- **Chrome extension and X**: side-panel splits and their browser/window actions, shared tabs, X replies,
  store sign-in and extension analytics: Codync builds no browser extension and has no X connector.
- **Credits/Billing**: animated credit slider: no credit wallet or Buy credits page.
- **Web client**: faster server connect, joined-server Tasks→Threads rename, slow-connection upload:
  no web client; Codync's desktop already calls the list Threads in the chat side panel.
- **Joined servers**: hiding a server across devices, stalled-server reconnect, provider version pages,
  account connection slots, live "Parallel threads per agent" setting, version-gated question text:
  Codync's host owns the logic and has no joined-server protocol; its relay is its own.
- **OS packaging**: random update restart delay, macOS dock flicker and iPhone Mirroring, Windows app
  ID, low-memory server browser limits: OpenBot's installers and drivers.
- **Performance-only internals**: fewer typing events, less CPU parsing streamed replies, shared step
  clocks and merged refreshes, server-side update CPU: no visible behavior; React render path differs.
- **Other**: Hermes `revision_conflict`, Pi output-scanner truncation, Codex Chrome plugins, Natural
  response style, X branding, "find the voice conversation" help prompt, channel message previews
  (Codync deliberately stores routing metadata only), shared `agent-data.db` tables (no such tool).
