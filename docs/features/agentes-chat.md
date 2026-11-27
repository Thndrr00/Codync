# Patterns taken from agentes.chat

agentes.chat (Codari Global, `AGPL-3.0-only`) is read-only reference: its engine is TypeScript over
Bun, so nothing is copied. Each item below is a **behavior** rewritten against Codync's own tables,
with the files where it lives and the tests that hold it. Its ADRs are read before designing; where
one decides differently from us and we keep ours, it is written down in [ADR deltas](agentes-chat-adr-deltas.md).
Its `LICENSE` and `NOTICE` stay behind if any line is ever lifted verbatim.

## What Codync already had

| Pattern | Already here |
|---|---|
| A1 Approval cards, one decision each | `PermissionCard`, `chat/*/approve` |
| A3 GitHub pull-request triggers | `host/src/github`, routine `pull_request` triggers |
| A6 A board of what the bots are doing | `botTasks` |

## What was reimplemented

| Pattern | What Codync does now | Code |
|---|---|---|
| A2 An approval is recorded once | Durable ledger with a fingerprint of the card (tool kind, command, folder, diffs); consuming it twice fails, and an automatic rule or the bot's automatic setting leaves its own record naming what let it through | `store/approvals.rs`, `agent/approval_ledger.rs` |
| A4 Pages that name each other | `[[Title]]` links, backlinks, and the graph of a library | `store/documents/links.rs` |
| A5 Every published file keeps its versions | `fileVersions` lists the snapshots of a shared name, newest first | `chat/library.rs` |
| A7 A mailbox through connected apps | Gmail and Outlook through the existing connectors, documented | `docs/features/mailbox.md` |
| A8 Questions answered from a database | Read-only SQLite, one statement, row/column/time caps (the deadline held by a real slow query); SQLite only, with the reasons and what another engine would need | `sql.rs`, `docs/features/read-only-database.md` |
| A9 A takeover handed back with a note | `resolveTakeover` ends the takeover with the reason the agent resumes from; taking it records who and since when and holds the agent's queue | `chat/takeover.rs`, `hub.rs` |
| A10 An import says what it leaves behind | The credential report counts keys a person must re-enter | `import/manifest.rs` |
| An export a person can open elsewhere | HTML, Word or PDF through the machine's converters, naming the one to install when there is none | `document_export.rs`, `docs/features/opendots-documents.md` |
| An export uploaded to object storage | A path-style, SigV4-signed `PUT` to the bucket set on the computer | `s3.rs` |
| A11 A rule learned from two refusals | Two identical denials propose one deny rule, never applied by itself | `agent/approval_lessons.rs` |
| A12 A run at the token limit | Holds (`wait`) or skips per the routine's `limit_policy`; test runs are exempt | `routines/runner.rs` |
| Search over what a bot knows | One index, two kinds (message, document), keys masked, internal entries out, resumable backfill | `store/search.rs`, `chat/recall.rs` |
| A person waiting goes first (ADR 0046) | A bot's queue has two lanes: interactive turns (a person or another bot, kept in strict FIFO as Codync promises clients) and background work (routines, compactions), which goes behind them and drops to the interactive lane after two minutes so it never starves | `agent/bot/cmd.rs`, `agent/bot/queue.rs` |
| Notes don't loop (ADR 0048) | `message_bot` drops a note whose normalized text was already sent to the same bot in the last hour; only notes that got out are remembered | `chat/team/requests.rs`, `chat/team.rs` |
| Quiet routines hand off (ADR 0047) | A routine's final text stays in its run history; with real news it calls `hand_off` (≤8000 chars, last one wins) and a conversation run takes it once via `routineHandoff` and says it in the agent's voice | `routines/handoff.rs`, `routines/mcp.rs`, `api/bots.rs` |
| A profile per person (ADR 0052) | Bounded facts about the person (30 facts, 200 chars each, 1200 total), shared by bots, inspectable and deletable through `personFacts`, and rendered as a hidden block for the run | `chat/person.rs`, `api/bots.rs` |
| One worktree per mission (ADR 0044) | A mission's branch and folder are derived once from its id and title, safe for git and disks; `missionWorktree` returns the plan | `chat/mission_worktree.rs`, `api/bots.rs` |
| Tools per run kind (ADR 0043) | A routine run's tool set is declared in one place and every withheld tool carries the reason to tell the agent (`toolSet`); a conversation keeps all tools | `chat/toolset.rs`, `api/bots.rs` |
| Choice cards (ADR 0124) | A bounded question with 2–8 labeled options, answered once through `choiceCard`; a later answer cannot change the first | `chat/choices.rs`, `api/bots.rs` |
| Message links (ADR 0138) | One permalink per message (`codync://message/<bot>/<entry>`) and a structured cite so clients draw a pill and agents point to where they were asked | `chat/message_link.rs`, `api/bots.rs` |
| How agents talk (ADR 0147) | A per-bot voice, simple by default or technical by choice, that only changes the wording line through `agentVoice` | `chat/voice.rs`, `api/bots.rs` |
| Presence and written status (ADR 0084) | A bounded, expiring status a person writes for a bot, shown next to its runtime status through `botPresence` | `chat/presence.rs`, `api/bots.rs` |
| Setup checklist (ADR 0155/0164) | A new bot's chat shows only what is missing — agent, instructions, folder — through `botSetup` | `chat/setup.rs`, `api/bots.rs` |
| Stretches (ADR 0089) | Work agents do on their own is bounded by runs and budget share, with one `Continuar` card through `stretchCard`; Codync keeps its own hop limits | `chat/stretch.rs`, `api/bots.rs` |
| Approval requests by agents (ADR 0126) | A bounded request with who asked, why and two options, answered once through `agentApproval` | `chat/agent_request.rs`, `api/bots.rs` |
| Local vault (ADR 0144) | An index of secret names, sources and masked hints; the value stays in the OS Keychain, never in the index, through `vaultMask` | `chat/vault.rs`, `api/bots.rs` |

| Tombstone on delete (ADR 0082) | A deleted agent's record names the memory, files and routines it left, through `agentTombstone` | `chat/agent_tombstone.rs`, `api/bots.rs` |
| Messages in a row (ADR 0112) | A quick burst is one request, not several turns, through `burstRequest` | `chat/burst.rs`, `api/bots.rs` |
| Check before catch-up (ADR 0136) | A catch-up after a usage pause first checks the pause ended, through `catchUpCheck` | `chat/catchup_check.rs`, `api/bots.rs` |
| Back to latest (ADR 0140) | Resuming opens at the newest messages, with a bounded backtrack, through `resumeAnchor` | `chat/latest.rs`, `api/bots.rs` |
| Turn verdicts (ADR 0097) | One vocabulary for how a turn ends — done, stopped, failed, waiting — through `turnVerdict` | `chat/continuation.rs`, `api/bots.rs` |
| Memory that proves itself (ADR 0167) | A claim is recorded only when a call produced it and names its source, through `claimAdmissible` | `chat/provenance.rs`, `api/bots.rs` |

| Agents may pass (ADR 0049) | Silence is a normal `(pass)` outcome, not a failure, through `agentPass` | `chat/pass.rs`, `api/bots.rs` |
| Context window (ADR 0086) | The reported used/total tokens map to the existing compaction threshold, through `contextWindow` | `chat/context_window.rs`, `api/bots.rs` |
| Team coordinator (ADR 0093) | The lead coordinates; members hand results back, chosen from the roster through `teamCoordinator` | `chat/coordinator.rs`, `api/bots.rs` |
| Hand-offs come back (ADR 0095) | A hand-off result is tagged as from another bot, never a person's words, through `handoffResult` | `chat/handback.rs`, `api/bots.rs` |
| Several accounts per connector (ADR 0099) | A reply uses the account its message named, never a wrong one, through `connectorAccount` | `chat/connector_accounts.rs`, `api/bots.rs` |
| Sign-in per connection (ADR 0150) | The chosen sign-in method is per connection and the secret is never echoed, through `signInMethod` | `chat/signin_method.rs`, `api/bots.rs` |

| Agents propose agents (ADR 0081) | An agent proposes a bounded bot card; a person creates it, through `proposedAgent` | `chat/proposed_agent.rs`, `api/bots.rs` |
| One setup, many agents (ADR 0094) | A setup copies only its shared fields onto a target, keeping its identity, through `sharedSetup` | `chat/shared_setup.rs`, `api/bots.rs` |
| Resourceful agents (ADR 0109) | A single failure is retried; giving up needs repeated, evidenced failure, through `resourceful` | `chat/resourceful.rs`, `api/bots.rs` |
| Live blocks (ADR 0113) | An agent shows a small non-persistent block to act on, through `liveBlock` | `chat/live_block.rs`, `api/bots.rs` |
| Agents start UI actions (ADR 0129) | An app action is also a host tool, named as a host call, through `uiAction` | `chat/ui_action.rs`, `api/bots.rs` |
| Personal mode (ADR 0159) | One member hides team surfaces without adding a team model, through `personalMode` | `chat/personal_mode.rs`, `api/bots.rs` |

## Newer ADRs brought from the updated repo

After fetching the reference repo (it was 12,427 commits behind), these newer behaviors were added
as host modules with unit tests and API methods, reusing their rules rather than copying code:
**0173** approvals level per bot (always/risky/never), **0176** `keep_running` holds, **0207** the
one-line intent before slow work, **0254/0256** time per ask by kind and by stage, and **0257**
maintenance words instead of a bare "Offline". They live in `chat/{approvals_level,keep_running,
intent_line,ask_metrics,computer_maintenance}.rs`.

A second pass over the newer ADRs added six more behaviors, also as host modules with tests and API
methods: **0219** reviewed, undoable improvements, **0241** computer down/back notices, **0242** a
routine offered when connecting an app, **0243** one bounded /tmp memory pool with pressure actions,
**0250** message origin (only typed words are the person's own) and **0255** one OS-permission wait
told everywhere. They live in `chat/{improvement,outage_notice,connect_routine,tmp_budget,
message_origin,os_permission}.rs`.

A third pass added **0172** "waiting on you" as a view of the inbox (not a second list), **0182**
approvals running in the order asked, and **0228** one "anyone looking" state for quiet work. They
live in `chat/{waiting_on_you,approval_order,sight}.rs`.

## In B and C, fully brought in

- **ADR 0089 (stretches)** is wired into `routines::runner::begin`: a routine's chain of unattended runs stops at the caps (10 unattended / 50 interactive) and the run is cancelled, so the agent never runs forever on its own.
- **ADR 0047 (quiet handoff)** is available through `routineHandoff`: a routine's final text is stored as a summary and a conversation run takes it once and says it in the agent's voice.
- **B's other behaviors** (choice cards, live blocks, presence, setup, stretches, approvals, vault) now have real views in all four clients: `AgentesChatPanel.tsx`, `AgentesChatView.swift`, `AgentesChatSheet.kt` and `tui/sheets/agentes_chat_view.rs`, each rendering only host answers.

The clients reach all of these through typed bridges that only call the host and decode its answer:
`apps/desktop/src/renderer/client/agentes-chat.ts`, `CodyncKit/Client/HostClient+AgentesChat.swift`,
`core/.../AgentesChat.kt` and `host/src/tui/net/agentes_chat.rs`. The host still owns the logic.

## Not taken

- **Embeddings**, organization permissions and the other search filters: no provider in the host and
  no organization model.
- From ADR 0046, **three global priority levels and reserved slots**: Codync queues turns per bot,
  not jobs in one global outbox, so there are no shared slots to reserve, and it promises FIFO across
  senders — a bot's message cannot overtake a person's. The "person first" idea is kept as two lanes
  (interactive before background) with the two-minute aging; the three levels are collapsed.

Built since this report was written: **missions** are their own feature now
([missions](../features/missions.md), block 1), GitHub pull-request work runs from a **GitHub App**
([github-app](../features/github-app.md), block 4), a bot works in a **brain**
([brains](brains.md), block 3), and a **module** is the extension point
([modules](../architecture/modules.md), block 5), so none of them is a departure from agentes.chat
any more.

The plan for the rest is `docs/plan-agentes-chat.md`.
