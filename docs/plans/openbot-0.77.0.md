# OpenBot 0.77.0 in Codync

Source: OpenBot nightly tag `v0.77.0` (329 changelog bullets), read from `CHANGELOG.md`.
Same rule as the earlier plans: copy behavior and user-facing text with a short source comment,
reuse Codync's own components and bridge to Codync concepts where an OpenBot subsystem does not
exist. "Joined server", web-client and hosted-server items need a server model Codync doesn't have.

## State

Most of 0.77.0 was already integrated before this tag shipped, from the unreleased fragments on
OpenBot's `origin/main`: strict typed answers, path checks and the `workflow_answer` tool
(`fc546e3af`), versioned workflow format, pause, export/import, trigger fields and output messages
(`83db95c2f`), field chips and the canvas UI for them (`5ee10e780`), ghost/jellyfish/octopus
avatars, video controls, thread Back and older archived threads (`238733f54`), Gemini relist,
slow-account models, Codex Ask first and CJK memory search (`27a28f392`), and document paths,
answer placement, requests-ended state and subagent cards (`6190f45df`). The media batch,
availability and three-day reference retention from this tag landed in this branch as well.

The remaining portable items are being integrated in disjoint batches on this branch: idle
compaction and cache policy, the location-request tool, WhatsApp read-only chats and contact cards,
Gemini's own-tool permissions, browser sign-in/takeover cards, and the last workflow fixes.

Nothing in this plan has been compiled or run end to end on this machine.

## Ported before this tag shipped (fragments)

| OpenBot 0.77.0 | Codync | Where |
| --- | --- | --- |
| Strict `int`/`integer`, number and list answers; one retry with the problem; webhook field names checked | Pure validators in the flow answer path, one bounded retry | `host/src/routines/flow/answers.rs` |
| A step fails clearly when a supplier failed or a path doesn't give a field | The answer-path graph and fail-closed steps | `host/src/routines/flow/answer_paths.rs` |
| `workflow_answer` tool; Claude gets the exact answer shape | The tool declaration and validation, wired to the real agent call | `host/src/routines/flow/tools.rs` |
| `{{` completion, field chips with whole-chip Backspace, "Insert field", Answer fields editor | The canvas and its pure completion helpers | desktop `views/routines/flow/**`, `lib/routine-flow.ts` |
| Format version, read-only newer workflows, unknown settings refused | The versioned format and merge rules | `host/src/routines/flow/format.rs`, `merge.rs` |
| Pause a workflow; Duplicate makes a paused copy; export/import `.openbot.workflow` | The pause, portable format and canvas actions | `host/src/routines/flow/pause.rs`, `portable.rs` |
| Trigger steps give fixed fields to later steps | The trigger-field projection | `host/src/routines/flow/trigger_fields.rs` |
| Output step message with `{{name}}` values | The output template | `host/src/routines/flow/output_template.rs` |
| Ghost, jellyfish and octopus avatars; video controls and full size; thread Back | All clients | clients' avatar, video and thread views |
| Second account's models, Codex Ask first, 10-minute Gemini relist, CJK memory search | Host agent and auth paths | `host/src/agent/**` |
| Up to four media results per step, credits on Run, progress and Retry, unavailable model, three-day references | Pure batch validators and the existing generate path | `host/src/media/batch.rs`, desktop flow views |

## Remaining portable items

| OpenBot 0.77.0 | Codync bridge |
| --- | --- |
| Idle Claude or Pi chat, task or channel thread with 200K tokens compacts itself | Pure idle rule and the bot actor's compaction path, extended to task/channel lanes |
| "Compact before the cache expires" switch, on by default; 1-hour cache for Pi | Per-bot switch in the compaction row and the cache-life decision |
| Location request tool | Built-in tool plus an answerable card on the existing approval path |
| Read-only allowed WhatsApp chat; contact cards; group replies to the orchestrator | The WhatsApp settings list, incoming parse and existing bot route |
| Gemini Full access uses Codync's own tools; Workspace-only still messages agents | The existing one-place permission decision |
| Inactive sign-in, stalled page and takeover cards | The browser tools and shared card views |
| Loop rounds, "Queued" step, run-one-step staleness, result rows | The flow engine and canvas |

## Not ported — no Codync equivalent

- **Credits and billing**: buying credits, free monthly credits, credit activity, audit log, account-service
  analytics and the credits card in the server menu. Codync has no credit wallet, Stripe billing or plans.
- **Hosted and joined servers**: server-menu cards, server transfers, joined-server version gates, remote
  sign-in to a server, server-side picture handling, server restarts and update policies. Codync's host
  owns the logic and has no joined-server protocol; its relay and certificate stack are its own.
- **Signal server selection**: Codync uses its own relay (`host/src/remote/relay.rs`) rather than choosing
  among OpenBot's regional Signal servers.
- **Web client**: attachment upload, browser-only sign-in card text and web-only state cards. Codync has
  no web client.
- **SolidJS 2.0.3**: Codync's desktop renderer is React/TypeScript.
- **OS packaging and permissions**: AppArmor, Debian/Raspberry Pi OS, Windows Smart App Control,
  EBUSY/EPIPE, notification center, keyring, macOS temp-copy updater, display sleep, Calendar/Reminders
  prompts, Wayland/Hyprland and GPU-specific fixes. These are OpenBot's installers and drivers.
- **Other products**: CRM form fixes, Docker guide and server install command, Datadog EU app,
  Muse/Meta key page, OpenBot 101 guides, Hermes/Kiro/Devin/Cursor CLI specifics where Codync has no
  such backend.
